use anyhow::{Context, Result, anyhow};

use crate::hash::fnv1a_hash;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PooledString {
    /// The bucket where this pooled string is
    bucket: u16,

    /// The offset inside the bucket.
    offset: u16,
}

struct StringPoolItem {
    value: String,

    /// Unique among all StringPoolItem in the pool.
    id: PooledString,
}

/// A bucket consist of a list of bucket chunks.
/// We can thing of the chunk as a linked list where each node has an
/// array items.
struct StringPoolBucket {
    data: Vec<StringPoolItem>,
}

// For now we hardcode the capacity to have 1024 buckets each with
// an initial chunk of 64, this allows us to store 64*1024=65536
// different strings.
//
// Proabbly we will need to better adapt this if we want to store eg: millions of different strings.
//
// However, for our purposes we know this is more than enough.
//
// BUCKETS_COUNT must be a power of 2.
//
// Also, both, BUCKETS_COUNT and BUCKET_CHUNK_SIZE fits into 16 bits.
// As PooledString indexes have that size in order to save memory.
//
// Therefore, the maximum value for both is 65535. If you need to get even closer
// to those numbers probably you are using this data structure wrong.
const BUCKETS_COUNT: usize = 1024;

// The amount elements that can fill into a bucket.
const BUCKET_SIZE: usize = 64;

pub struct StringPool {
    buckets: Vec<StringPoolBucket>,
}

impl StringPool {
    pub fn new() -> StringPool {
        let mut buckets: Vec<StringPoolBucket> = Vec::with_capacity(BUCKETS_COUNT);

        for _ in 0..BUCKETS_COUNT {
            let bucket = StringPoolBucket { data: Vec::new() };
            buckets.push(bucket);
        }

        StringPool { buckets }
    }

    /**
     * Note we receive an String instead of an string slice or an slice of bytes
     * this is because the pool takes ownership over the pooled string and returns
     * a PooledString which is an ID to locate the owned string in the pool.
     */
    pub fn add_string_to_pool(&mut self, text: String) -> Result<PooledString> {
        let text_bytes = &text.as_bytes();
        let text_hash = fnv1a_hash(text_bytes) as usize;

        let bucket_index = text_hash & (BUCKETS_COUNT - 1);

        let bucket = self.buckets.get_mut(bucket_index).context(
            "cannot find bucket in StringPool, probably BUCKETS_COUNT is not a power of 2",
        )?;

        // Worst case when collisions happens this is O(n) search in the bucket items for the
        // text received as argument.
        let bucket_items: &Vec<StringPoolItem> = bucket.data.as_ref();
        for candidate_item in bucket_items {
            if candidate_item.value == text {
                return Ok(candidate_item.id);
            }
        }

        // Did not found an existing PooledString for received string, adding it.

        // Checks if bucket chunk has room for the new item.
        if bucket.data.len() + 1 > BUCKET_SIZE {
            // In the future here, we can grow the bucket count to the next multiple
            // of 2.
            return Err(anyhow!("bucket {bucket_index} is full"));
        }

        let pooled_string = PooledString {
            bucket: bucket_index as u16,
            offset: bucket.data.len() as u16,
        };

        bucket.data.push(StringPoolItem {
            value: text,
            id: pooled_string,
        });

        Ok(pooled_string)
    }

    /// Receives a string slice, and returns their corresponding PooledString
    /// if found in the pool.
    ///
    /// If not found, return Option::None.
    ///
    /// Note we receive an string slice as this do no takes ownership over the passed argument.
    pub fn find_pooled_string(&self, text: &str) -> Option<PooledString> {
        let text_bytes = &text.as_bytes();
        let text_hash = fnv1a_hash(text_bytes) as usize;

        let bucket_index = text_hash & (BUCKETS_COUNT - 1);
        let bucket = self.buckets.get(bucket_index)?;

        // O(n) search in the chunk for the text reeived as argument.
        let bucket_items: &Vec<StringPoolItem> = bucket.data.as_ref();
        for candidate_item in bucket_items {
            if candidate_item.value == text {
                return Some(candidate_item.id);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::string_pool::{PooledString, StringPool};
    use anyhow::Result;

    #[test]
    fn find_should_return_none_if_not_added_yet() {
        let pool = StringPool::new();
        assert_eq!(pool.find_pooled_string("Hello Text"), Option::None);
    }

    #[test]
    fn pooled_string_should_be_stable() -> Result<()> {
        let mut pool = StringPool::new();

        let pooled_string_1 = pool.add_string_to_pool(String::from("hello"))?;
        let pooled_string_2 = pool.add_string_to_pool(String::from("hello"))?;

        assert_eq!(pooled_string_1, pooled_string_2);
        Ok(())
    }

    #[test]
    fn pooled_string_is_case_sensitive() -> Result<()> {
        let mut pool = StringPool::new();

        let pooled_string_1 = pool.add_string_to_pool(String::from("HELLO"))?;
        let pooled_string_2 = pool.add_string_to_pool(String::from("hello"))?;

        assert_ne!(pooled_string_1, pooled_string_2);

        Ok(())
    }

    #[test]
    fn find_should_return_same_pooled_string() -> Result<()> {
        let mut pool = StringPool::new();

        assert_eq!(pool.find_pooled_string("hello"), Option::None);
        pool.add_string_to_pool(String::from("hello"))?;
        pool.add_string_to_pool(String::from("hello"))?;

        let expected_pooled_string = PooledString {
            bucket: 267,
            offset: 0,
        };

        if let Some(found_pooled_string) = pool.find_pooled_string("hello") {
            assert_eq!(found_pooled_string, expected_pooled_string);
        } else {
            panic!("should find a pooled string, but not found");
        }

        Ok(())
    }
}
