
use crate::MoaResult;
use std::collections::{HashMap, HashSet};

pub fn jaccard_distance(text1: &str, text2: &str) -> f32 {
    let words1: HashSet<&str> = text1.split_whitespace().collect();
    let words2: HashSet<&str> = text2.split_whitespace().collect();
    
    let intersection_size = words1.intersection(&words2).count();
    let union_size = words1.union(&words2).count();
    
    if union_size == 0 {
        return 0.0;
    }
    
    1.0 - (intersection_size as f32 / union_size as f32)
}

pub fn cosine_similarity(text1: &str, text2: &str) -> MoaResult<f32> {
    let words1 = tokenize_and_count(text1);
    let words2 = tokenize_and_count(text2);
    
    let mut dot_product = 0.0;
    let mut norm1 = 0.0;
    let mut norm2 = 0.0;
    
    let all_words: HashSet<_> = words1.keys().chain(words2.keys()).collect();
    
    for word in all_words {
        let count1 = *words1.get(word).unwrap_or(&0) as f32;
        let count2 = *words2.get(word).unwrap_or(&0) as f32;
        
        dot_product += count1 * count2;
        norm1 += count1 * count1;
        norm2 += count2 * count2;
    }
    
    if norm1 == 0.0 || norm2 == 0.0 {
        return Ok(0.0);
    }
    
    Ok(dot_product / (norm1.sqrt() * norm2.sqrt()))
}

fn tokenize_and_count(text: &str) -> HashMap<String, usize> {
    let mut word_count = HashMap::new();
    
    for word in text.split_whitespace() {
        let clean_word = word.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        
        if !clean_word.is_empty() {
            *word_count.entry(clean_word).or_insert(0) += 1;
        }
    }
    
    word_count
}

pub fn semantic_similarity(text1: &str, text2: &str) -> MoaResult<f32> {
    // Placeholder for more advanced semantic similarity
    // In practice, this would use embeddings
    cosine_similarity(text1, text2)
}