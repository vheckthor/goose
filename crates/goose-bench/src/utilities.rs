use std::collections::HashMap;

pub fn union_hashmaps<K, V>(maps: Vec<HashMap<K, V>>) -> HashMap<K, V>
where
    K: Eq + std::hash::Hash,
    V: Clone,
{
    // We can use the fold method to accumulate all maps into one
    maps.into_iter().fold(HashMap::new(), |mut result, map| {
        // For each map in the vector, extend the result with its entries
        result.extend(map);
        result
    })
}
