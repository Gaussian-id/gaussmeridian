#!/bin/bash
# GaussMeridian M2-VAL Behavioral Validation Suite
# Usage: BASE_URL=http://localhost:8080 API_KEY=<key> ./scripts/validate-routing.sh
#
# Checks X-GaussMeridian-Tier and X-GaussMeridian-Complexity response headers.
# Requires a live GaussMeridian server. See Sprint/Developer Guide for startup instructions.
#
# NOTE: This script CANNOT be run as part of `cargo test`. It requires a live
# GaussMeridian server process. Start the server first:
#   cargo run --bin gaussmeridian
# Then run this script from the repository root:
#   BASE_URL=http://localhost:8080 API_KEY=your-key ./scripts/validate-routing.sh

set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
API_KEY="${API_KEY:-test-key}"
LOG_FILE="validation-$(date +%Y%m%d-%H%M%S).log"
PASS=0
FAIL=0

log() { echo "$1" | tee -a "$LOG_FILE"; }

check() {
    local label="$1"
    local content="$2"
    local expect_tier="$3"
    local expect_complexity_op="$4"   # "lt" or "gt"
    local expect_complexity_val="$5"

    response=$(curl -s -D - -o /dev/null -X POST "$BASE_URL/v1/chat/completions" \
        -H "Authorization: Bearer $API_KEY" \
        -H "Content-Type: application/json" \
        -d "{\"model\":\"auto\",\"messages\":[{\"role\":\"user\",\"content\":\"$content\"}]}" 2>/dev/null || echo "CURL_FAILED")

    if [[ "$response" == "CURL_FAILED" ]]; then
        log "[FAIL] $label — curl failed (server unreachable?)"
        ((FAIL++))
        return
    fi

    tier=$(echo "$response" | grep -i "x-gaussmeridian-tier" | awk -F': ' '{print $2}' | tr -d '\r\n' || echo "")
    complexity=$(echo "$response" | grep -i "x-gaussmeridian-complexity" | awk -F': ' '{print $2}' | tr -d '\r\n' || echo "0")

    tier_ok=false
    [[ "$tier" == "$expect_tier" ]] && tier_ok=true

    complexity_ok=false
    if [[ -n "$complexity" && "$complexity" != "0" ]]; then
        if [[ "$expect_complexity_op" == "lt" ]]; then
            result=$(python3 -c "print('ok' if float('${complexity}') < float('${expect_complexity_val}') else 'fail')" 2>/dev/null || echo "fail")
        else
            result=$(python3 -c "print('ok' if float('${complexity}') > float('${expect_complexity_val}') else 'fail')" 2>/dev/null || echo "fail")
        fi
        [[ "$result" == "ok" ]] && complexity_ok=true
    fi

    if $tier_ok && $complexity_ok; then
        log "[PASS] $label | tier=$tier | complexity=$complexity"
        ((PASS++))
    else
        log "[FAIL] $label | tier=$tier (expected=$expect_tier, ok=$tier_ok) | complexity=$complexity (expected $expect_complexity_op $expect_complexity_val, ok=$complexity_ok)"
        ((FAIL++))
    fi
}

log "=== GaussMeridian M2-VAL Behavioral Validation ==="
log "Server: $BASE_URL"
log "Log: $LOG_FILE"
log ""

log "--- GROUP 1: Simple queries → efficient tier ---"
check "G1-1: Capital city"      "What is the capital of France?"                      efficient lt 0.30
check "G1-2: Basic math"        "What is 12 times 8?"                                 efficient lt 0.30
check "G1-3: Simple greeting"   "Hello, how are you today?"                           efficient lt 0.20
check "G1-4: Unit conversion"   "How many centimeters in an inch?"                    efficient lt 0.25
check "G1-5: Days in year"      "How many days are in a leap year?"                   efficient lt 0.30
check "G1-6: Definition"        "What is machine learning in one sentence?"           efficient lt 0.35

log ""
log "--- GROUP 2: Code queries → specialist tier ---"
check "G2-1: Rust binary search"    "Write a Rust function to implement binary search on a sorted Vec"                       specialist gt 0.40
check "G2-2: Python debug"          "Debug this Python: def fib(n): return fib(n-1) + fib(n-2)"                             specialist gt 0.35
check "G2-3: Architecture"          "Design a Redis-backed sliding window rate limiter in Go"                                 specialist gt 0.45
check "G2-4: SQL review"            "Review this SQL query for performance issues: SELECT * FROM orders WHERE status = open" specialist gt 0.35
check "G2-5: Implicit fix"          "Fix this: TypeError: Cannot read property map of undefined"                              specialist gt 0.30
check "G2-6: Dijkstra"              "Implement Dijkstra shortest path in TypeScript"                                          specialist gt 0.45
check "G2-7: Unit tests"            "Write unit tests for a function that validates email addresses"                          specialist gt 0.35
check "G2-8: Regex"                 "Write a regex to extract ISO 8601 dates from a log file"                                 specialist gt 0.30

log ""
log "--- GROUP 3: Complex multi-domain queries → flagship tier ---"
check "G3-1: GDPR legal"     "Analyse the jurisdictional implications of cross-border data transfers under GDPR Article 46(2)(c) for a multi-party SaaS" flagship gt 0.70
check "G3-2: Medical"        "What are the contraindications for combining SSRIs with MAOIs in elderly patients with renal impairment?"                   flagship gt 0.65
check "G3-3: Financial"      "Compare the amortization treatment of goodwill under IFRS 3 versus ASC 350 and explain the audit implications"              flagship gt 0.65
check "G3-4: Multi-domain"   "Explain the legal and technical compliance requirements for a fintech startup processing EU payment data under PSD2 and GDPR simultaneously" flagship gt 0.75
check "G3-5: Scientific"     "Explain the quantum decoherence problem and its implications for error correction in topological quantum computing"          flagship gt 0.65
check "G3-6: Ethical"        "Analyse the ethical tradeoffs of algorithmic sentencing systems in criminal justice under Rawlsian vs utilitarian frameworks" flagship gt 0.70
check "G3-7: Strategic"      "What are the strategic implications of the EU AI Act for a B2B SaaS company with ML-powered features targeting regulated industries?" flagship gt 0.60
check "G3-8: Multi-statute"  "What is the liability exposure of a UK-based SaaS provider if their AI system produces discriminatory outputs and under which statutes" flagship gt 0.70

log ""
log "--- GROUP 4: Edge cases — legacy advisory-skill paraphrase coverage ---"
check "G4-1: Implicit legal"       "What is my liability if a contractor gets hurt on my property?"                            flagship gt 0.40
check "G4-2: Implicit code"        "make it work: const x = arr.find(i => i.id === target)"                                    specialist gt 0.30
check "G4-3: Medication implicit"  "What medication should I take for a persistent headache?"                                   flagship gt 0.40
check "G4-4: Numeric but simple"   "How many days in a year?"                                                                   efficient lt 0.30
check "G4-5: Translation"          "Translate this paragraph to French: The router selects models based on complexity"          efficient lt 0.35
check "G4-6: Entity extraction"    "Find all company names mentioned in this paragraph: Apple Microsoft and Google announced"    efficient lt 0.40
check "G4-7: Regulatory implicit"  "Is our data processing compliant with the new regulations?"                                 flagship gt 0.45
check "G4-8: Debugging medical"    "The patient potassium levels are dropping after starting the diuretic — what is happening?"  flagship gt 0.55

log ""
log "=== RESULTS: $PASS passed, $FAIL failed out of $((PASS + FAIL)) queries ==="
log "Full log: $LOG_FILE"

# Threshold check
g1_failures=$(grep -c "^\[FAIL\] G1" "$LOG_FILE" || true)
g3_failures=$(grep -c "^\[FAIL\] G3" "$LOG_FILE" || true)
g4_failures=$(grep -c "^\[FAIL\] G4" "$LOG_FILE" || true)

if [[ $g1_failures -gt 0 ]]; then
    log "ERROR: G1 (simple) failed $g1_failures/6 — all must pass"
    exit 1
fi
if [[ $g3_failures -gt 0 ]]; then
    log "ERROR: G3 (flagship) failed $g3_failures/8 — all must pass"
    exit 1
fi
if [[ $g4_failures -gt 3 ]]; then
    log "WARNING: G4 (edge cases) failed $g4_failures/8 — advisory-skill keyword coverage may need a second pass"
    exit 1
fi

log "All required thresholds met."
exit 0
