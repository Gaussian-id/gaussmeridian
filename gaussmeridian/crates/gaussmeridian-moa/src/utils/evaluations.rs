use crate::{models, MoaResult};
use std::collections::HashMap;

pub struct EvaluationMetrics {
    pub rouge_1: f32,
    pub rouge_2: f32,
    pub rouge_l: f32,
    pub bleu_score: f32,
    pub coherence_score: f32,
    pub diversity_score: f32,
}

pub fn evaluate_response(
    response: &models::MoaResponse,
    reference: Option<&str>
) -> MoaResult<EvaluationMetrics> {
    let rouge_1 = if let Some(ref_text) = reference {
        calculate_rouge_1(&response.content, ref_text)
    } else {
        0.0
    };
    
    let rouge_2 = if let Some(ref_text) = reference {
        calculate_rouge_2(&response.content, ref_text)
    } else {
        0.0
    };
    
    let rouge_l = if let Some(ref_text) = reference {
        calculate_rouge_l(&response.content, ref_text)
    } else {
        0.0
    };
    
    let bleu_score = if let Some(ref_text) = reference {
        calculate_bleu(&response.content, ref_text)
    } else {
        0.0
    };
    
    let coherence_score = calculate_coherence(&response.content);
    let diversity_score = calculate_response_diversity(&response.agent_responses);
    
    Ok(EvaluationMetrics {
        rouge_1,
        rouge_2,
        rouge_l,
        bleu_score,
        coherence_score,
        diversity_score,
    })
}

fn calculate_rouge_1(candidate: &str, reference: &str) -> f32 {
    let candidate_words: HashMap<String, usize> = tokenize_and_count(candidate);
    let reference_words: HashMap<String, usize> = tokenize_and_count(reference);
    
    let mut overlap = 0;
    let mut total_ref_words = 0;
    
    for (word, count) in &reference_words {
        total_ref_words += count;
        if let Some(candidate_count) = candidate_words.get(word) {
            overlap += (*count).min(*candidate_count);
        }
    }
    
    if total_ref_words == 0 {
        0.0
    } else {
        overlap as f32 / total_ref_words as f32
    }
}

fn calculate_rouge_2(candidate: &str, reference: &str) -> f32 {
    let candidate_bigrams = extract_bigrams(candidate);
    let reference_bigrams = extract_bigrams(reference);
    
    let mut overlap = 0;
    let total_ref_bigrams = reference_bigrams.len();
    
    for bigram in &reference_bigrams {
        if candidate_bigrams.contains(bigram) {
            overlap += 1;
        }
    }
    
    if total_ref_bigrams == 0 {
        0.0
    } else {
        overlap as f32 / total_ref_bigrams as f32
    }
}

fn calculate_rouge_l(candidate: &str, reference: &str) -> f32 {
    let candidate_words: Vec<&str> = candidate.split_whitespace().collect();
    let reference_words: Vec<&str> = reference.split_whitespace().collect();
    
    let lcs_length = longest_common_subsequence(&candidate_words, &reference_words);
    
    if reference_words.is_empty() {
        0.0
    } else {
        lcs_length as f32 / reference_words.len() as f32
    }
}

fn calculate_bleu(candidate: &str, reference: &str) -> f32 {
    // Simplified BLEU-1 calculation
    calculate_rouge_1(candidate, reference)
}

fn calculate_coherence(text: &str) -> f32 {
    let sentences: Vec<&str> = text.split('.').filter(|s| !s.trim().is_empty()).collect();
    
    if sentences.len() < 2 {
        return 1.0;
    }
    
    let mut coherence_sum = 0.0;
    
    for i in 0..sentences.len() - 1 {
        let similarity = crate::utils::similarity::cosine_similarity(
            sentences[i].trim(),
            sentences[i + 1].trim()
        ).unwrap_or(0.0);
        coherence_sum += similarity;
    }
    
    coherence_sum / (sentences.len() - 1) as f32
}

fn calculate_response_diversity(responses: &[models::AgentResponse]) -> f32 {
    if responses.len() < 2 {
        return 0.0;
    }
    
    let mut total_distance = 0.0;
    let mut comparisons = 0;
    
    for i in 0..responses.len() {
        for j in (i + 1)..responses.len() {
            let distance = crate::utils::similarity::jaccard_distance(
                &responses[i].content,
                &responses[j].content
            );
            total_distance += distance;
            comparisons += 1;
        }
    }
    
    total_distance / comparisons as f32
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

fn extract_bigrams(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut bigrams = Vec::new();
    
    for i in 0..words.len().saturating_sub(1) {
        let bigram = format!("{} {}", words[i], words[i + 1]);
        bigrams.push(bigram);
    }
    
    bigrams
}

fn longest_common_subsequence(seq1: &[&str], seq2: &[&str]) -> usize {
    let m = seq1.len();
    let n = seq2.len();
    
    if m == 0 || n == 0 {
        return 0;
    }
    
    let mut dp = vec![vec![0; n + 1]; m + 1];
    
    for i in 1..=m {
        for j in 1..=n {
            if seq1[i - 1] == seq2[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    
    dp[m][n]
}