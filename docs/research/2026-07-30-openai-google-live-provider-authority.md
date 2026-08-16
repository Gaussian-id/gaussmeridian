---
title: OpenAI and Google Live Provider Authority
date: 2026-07-30
status: current
scope: live provider requalification envelope
evidence_class: first-party documentation
---

# OpenAI and Google Live Provider Authority

## Purpose

This note establishes the first-party model-ID, pricing, versioning, and deprecation authority for the following live requalification candidates as retrieved on 2026-07-30:

- `gpt-4o-mini`
- `gpt-4o`
- `gemini-2.5-flash-lite`
- `gemini-2.5-flash`
- `gemini-2.5-pro`

The evidence is documentation-level authority only. No provider credentials were accessed, and no authenticated provider request or account-entitlement check was performed. A model being documented does not prove that a particular account, region, project, quota tier, or capacity reservation can invoke it.

## Executive finding

The five requested model families remain documented by their providers, but they do not all meet the same reproducibility standard.

| Provider | Catalog model | Request-side qualification ID | Documented availability on 2026-07-30 | Strict immutable revision pin |
| --- | --- | --- | --- | --- |
| OpenAI | `gpt-4o-mini` | `gpt-4o-mini-2024-07-18` | Model page lists Chat Completions and Responses support | Yes |
| OpenAI | `gpt-4o` | Prefer `gpt-4o-2024-08-06` to match the alias's documented default snapshot; `gpt-4o-2024-11-20` is also listed | Catalog marks the family deprecated, while the model page still lists Chat Completions and Responses support; only the 2024-05-13 snapshot has a published shutdown date | Yes |
| Google | `gemini-2.5-flash-lite` | `gemini-2.5-flash-lite` | Listed as stable, but with an announced earliest shutdown date of 2026-10-16 | No public dated 2.5 revision ID |
| Google | `gemini-2.5-flash` | `gemini-2.5-flash` | Listed as stable, but with an announced earliest shutdown date of 2026-10-16 | No public dated 2.5 revision ID |
| Google | `gemini-2.5-pro` | `gemini-2.5-pro` | Listed as stable, but with an announced earliest shutdown date of 2026-10-16 | No public dated 2.5 revision ID |

The strict counterfactual qualification contract should therefore:

1. use dated OpenAI snapshots rather than the floating family aliases;
2. fail closed for the three Gemini 2.5 models if the contract requires an immutable request-side revision;
3. not silently treat Google's word `stable` as equivalent to an immutable dated snapshot;
4. record Google Models API metadata as observational provenance if the program later authorizes stable-ID qualification.

## Standard paid-tier pricing authority

All values below are USD per 1 million tokens. The table uses ordinary paid, synchronous inference: OpenAI's model-page text-token rates and Google's `Standard` paid tier. Batch, Flex, Priority, free-tier, tool-call, grounding, storage, and regional uplifts are not included in the headline rates.

| Model | Input | Cached input or cached-context processing | Output | Scope |
| --- | ---: | ---: | ---: | --- |
| `gpt-4o-mini` | $0.15 | $0.075 | $0.60 | Text tokens |
| `gpt-4o` | $2.50 | $1.25 | $10.00 | Text tokens |
| `gemini-2.5-flash-lite` | $0.10 | $0.01 | $0.40 | Text, image, or video input; output includes thinking tokens |
| `gemini-2.5-flash` | $0.30 | $0.03 | $2.50 | Text, image, or video input; output includes thinking tokens |
| `gemini-2.5-pro`, prompt at or below 200,000 tokens | $1.25 | $0.125 | $10.00 | Output includes thinking tokens |
| `gemini-2.5-pro`, prompt above 200,000 tokens | $2.50 | $0.25 | $15.00 | Output includes thinking tokens |

### Pricing caveats that must be encoded

- OpenAI's figures above are the text-token prices published on each model page. Tool calls and non-text modalities can create additional charges.
- OpenAI Batch and Priority processing have different published rates. Neither in-scope family appears in the current Flex table. A qualification run must record the actual processing mode instead of assuming the headline model-page rate.
- The OpenAI model pages list the free tier as unsupported for both in-scope models. Paid-account usage tiers also impose different request, token, and batch-queue limits.
- Google's `context caching price` is not an automatic generic cached-input discount. It applies to context caching and carries a separate storage price.
- Google charges context-cache storage at $1.00 per 1 million tokens per hour for Flash and Flash-Lite, and $4.50 per 1 million tokens per hour for Pro under the Standard paid tier.
- Google Flash audio input costs $1.00 per 1 million tokens and cached audio context costs $0.10. Flash-Lite audio input costs $0.30 and cached audio context costs $0.03.
- Google's output prices include thinking tokens. The recorded provider usage must therefore retain thinking-token accounting where the response exposes it.
- Pro pricing changes when the prompt crosses 200,000 tokens. The pricing authority must be selected using the actual billable prompt length, not the nominal model context window.
- Google Search and Maps grounding can add per-prompt or per-query charges. Those charges are outside this token-price matrix.
- Free-tier prices must not be substituted for paid production economics.
- Provider prices are mutable external facts. A sealed run must retain the source URL, retrieval timestamp, currency, processing tier, modality, prompt-length tier, and effective price row.

### OpenAI processing-mode prices

These are the current OpenAI text-token rows in USD per 1 million tokens:

| Model | Processing mode | Input | Cached input | Output |
| --- | --- | ---: | ---: | ---: |
| `gpt-4o-mini` | Standard | $0.15 | $0.075 | $0.60 |
| `gpt-4o-mini` | Batch | $0.075 | Not separately published | $0.30 |
| `gpt-4o-mini` | Priority | $0.25 | $0.125 | $1.00 |
| `gpt-4o` | Standard | $2.50 | $1.25 | $10.00 |
| `gpt-4o` | Batch | $1.25 | Not separately published | $5.00 |
| `gpt-4o` | Priority | $4.25 | $2.125 | $17.00 |

`Not separately published` does not mean that cached tokens are free. The live economics record must use the provider's actual billing treatment.

OpenAI prompt caching is automatic for eligible exact-prefix matches of at least 1,024 tokens. Only tokens reported by the provider as `cached_tokens` qualify for cached-input pricing; a repeated request must not be priced as cached merely because the application expected a cache hit.

Primary authority:

- [OpenAI API pricing](https://developers.openai.com/api/docs/pricing), retrieved 2026-07-30.
- [OpenAI prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching), retrieved 2026-07-30.

### Google processing-mode prices

The compact rows below show text, image, or video prices as `input / cached context / output`, in USD per 1 million tokens. Google's Batch and Flex rows are identical for these models.

| Model and prompt tier | Standard | Batch or Flex | Priority |
| --- | ---: | ---: | ---: |
| `gemini-2.5-flash-lite` | $0.10 / $0.01 / $0.40 | $0.05 / $0.01 / $0.20 | $0.18 / $0.018 / $0.72 |
| `gemini-2.5-flash` | $0.30 / $0.03 / $2.50 | $0.15 / $0.03 / $1.25 | $0.54 / $0.054 / $4.50 |
| `gemini-2.5-pro`, prompt at or below 200,000 tokens | $1.25 / $0.125 / $10.00 | $0.625 / $0.125 / $5.00 | $2.25 / $0.225 / $18.00 |
| `gemini-2.5-pro`, prompt above 200,000 tokens | $2.50 / $0.25 / $15.00 | $1.25 / $0.25 / $7.50 | $4.50 / $0.45 / $27.00 |

Flash-Lite audio input and cached-context prices are $0.15 / $0.03 for Batch or Flex and $0.54 / $0.054 for Priority. Flash audio input and cached-context prices are $0.50 / $0.10 for Batch or Flex and $1.80 / $0.18 for Priority.

Priority cache storage is $1.80 per 1 million tokens per hour for Flash and Flash-Lite, and $8.10 for Pro. Standard, Batch, and Flex storage use the lower rates recorded above.

Google enables implicit caching by default for Gemini 2.5 and newer models. The current caching guide publishes a 2,048-token minimum for Flash and Pro but does not publish a Flash-Lite threshold in that table. Only cache hits reported by the provider, such as `usage.total_cached_tokens` on the documented Interactions API surface, justify cached-context economics.

Primary authority:

- [Google Gemini Developer API pricing](https://ai.google.dev/gemini-api/docs/pricing?hl=en), retrieved 2026-07-30.
- [Google context caching](https://ai.google.dev/gemini-api/docs/caching?hl=en), retrieved 2026-07-30.

## OpenAI authority

### `gpt-4o-mini`

OpenAI's current model page identifies:

- model ID: `gpt-4o-mini`;
- default snapshot: `gpt-4o-mini-2024-07-18`;
- Chat Completions: supported;
- Responses: supported;
- context window: 128,000 tokens;
- maximum output: 16,384 tokens;
- text-token price: $0.15 input, $0.075 cached input, and $0.60 output per 1 million tokens.

The page states that snapshots lock a specific model version and lists only `gpt-4o-mini-2024-07-18`. The current OpenAI deprecations page does not list that snapshot as deprecated.

Qualification verdict: use `gpt-4o-mini-2024-07-18`. Do not seal a run that requests only `gpt-4o-mini`, because the family alias can be remapped later even though its current documented default is known.

Primary authority:

- [OpenAI GPT-4o mini model page](https://developers.openai.com/api/docs/models/gpt-4o-mini), retrieved 2026-07-30.
- [OpenAI GPT-4o mini machine-readable Markdown](https://developers.openai.com/api/docs/models/gpt-4o-mini.md), retrieved 2026-07-30.
- [OpenAI API deprecations](https://developers.openai.com/api/docs/deprecations), retrieved 2026-07-30.

### `gpt-4o`

OpenAI's current model page identifies:

- model ID: `gpt-4o`;
- default snapshot: `gpt-4o-2024-08-06`;
- Chat Completions: supported;
- Responses: supported;
- context window: 128,000 tokens;
- maximum output: 16,384 tokens;
- text-token price: $2.50 input, $1.25 cached input, and $10.00 output per 1 million tokens.

The model page lists three snapshots:

- `gpt-4o-2024-11-20`;
- `gpt-4o-2024-08-06`;
- `gpt-4o-2024-05-13`.

OpenAI's all-model catalog labels the GPT-4o family `Deprecated`. The formal deprecations page announces shutdown of `gpt-4o-2024-05-13` on 2026-10-23 and recommends `gpt-5.6-sol`, but it does not publish a shutdown date for the `gpt-4o` alias, `gpt-4o-2024-08-06`, or `gpt-4o-2024-11-20` at retrieval time. The catalog and schedule therefore provide asymmetric evidence: the family has a deprecation label, while two listed snapshots have no published retirement date.

Qualification verdict: use `gpt-4o-2024-08-06` when the objective is to reproduce what the `gpt-4o` alias denotes on the retrieval date. A study may instead choose `gpt-4o-2024-11-20`, but the chosen snapshot must be explicit and must not be mixed with the 2024-08-06 results. Treat either as a deprecated-family comparator, not a new promotion target. Do not use `gpt-4o-2024-05-13` for a new long-lived baseline.

Primary authority:

- [OpenAI GPT-4o model page](https://developers.openai.com/api/docs/models/gpt-4o), retrieved 2026-07-30.
- [OpenAI GPT-4o machine-readable Markdown](https://developers.openai.com/api/docs/models/gpt-4o.md), retrieved 2026-07-30.
- [OpenAI all-model catalog](https://developers.openai.com/api/docs/models/all), retrieved 2026-07-30.
- [OpenAI API deprecations](https://developers.openai.com/api/docs/deprecations), retrieved 2026-07-30.

## Google authority

### Wire format

Google's GenerateContent reference defines the REST endpoint as:

```text
POST https://generativelanguage.googleapis.com/v1beta/{model=models/*}:generateContent
```

The REST resource form is therefore `models/{model-code}`. Google SDKs accept the model code without the `models/` prefix.

Examples for the in-scope models:

```text
models/gemini-2.5-flash-lite
models/gemini-2.5-flash
models/gemini-2.5-pro
```

Primary authority:

- [Google GenerateContent API reference](https://ai.google.dev/api/generate-content), retrieved 2026-07-30.
- [Google Models API reference](https://ai.google.dev/api/models), retrieved 2026-07-30.

### `gemini-2.5-flash-lite`

Google's model page lists `gemini-2.5-flash-lite` as the model code and its stable version. It supports caching, a 1,048,576-token input limit, and a 65,536-token output limit.

Standard paid pricing:

- text, image, or video input: $0.10;
- audio input: $0.30;
- text, image, or video cached context: $0.01;
- audio cached context: $0.03;
- output, including thinking tokens: $0.40;
- cache storage: $1.00 per 1 million tokens per hour.

Deprecation status: the stable ID remains documented, but Google's deprecations page announces 2026-10-16 as its earliest shutdown date and recommends `gemini-3.1-flash-lite`.

Primary authority:

- [Google Gemini 2.5 Flash-Lite model page](https://ai.google.dev/gemini-api/docs/models/gemini-2.5-flash-lite?hl=en), retrieved 2026-07-30.
- [Google Gemini Developer API pricing](https://ai.google.dev/gemini-api/docs/pricing?hl=en), retrieved 2026-07-30.
- [Google Gemini deprecations](https://ai.google.dev/gemini-api/docs/deprecations?hl=en), retrieved 2026-07-30.

### `gemini-2.5-flash`

Google's model page lists `gemini-2.5-flash` as the model code and its stable version. It supports caching, a 1,048,576-token input limit, and a 65,536-token output limit.

Standard paid pricing:

- text, image, or video input: $0.30;
- audio input: $1.00;
- text, image, or video cached context: $0.03;
- audio cached context: $0.10;
- output, including thinking tokens: $2.50;
- cache storage: $1.00 per 1 million tokens per hour.

Deprecation status: the stable ID remains documented, but Google's deprecations page announces 2026-10-16 as its earliest shutdown date and recommends `gemini-3.6-flash`.

Primary authority:

- [Google Gemini 2.5 Flash model page](https://ai.google.dev/gemini-api/docs/models/gemini-2.5-flash?hl=en), retrieved 2026-07-30.
- [Google Gemini Developer API pricing](https://ai.google.dev/gemini-api/docs/pricing?hl=en), retrieved 2026-07-30.
- [Google Gemini deprecations](https://ai.google.dev/gemini-api/docs/deprecations?hl=en), retrieved 2026-07-30.

### `gemini-2.5-pro`

Google's model page lists `gemini-2.5-pro` as the model code and its stable version. It supports caching, a 1,048,576-token input limit, and a 65,536-token output limit.

Standard paid pricing for prompts at or below 200,000 tokens:

- input: $1.25;
- cached context: $0.125;
- output, including thinking tokens: $10.00.

Standard paid pricing for prompts above 200,000 tokens:

- input: $2.50;
- cached context: $0.25;
- output, including thinking tokens: $15.00.

Cache storage is $4.50 per 1 million tokens per hour.

Deprecation status: the stable ID remains documented, but Google's deprecations page announces 2026-10-16 as its earliest shutdown date and recommends `gemini-3.1-pro-preview`.

Primary authority:

- [Google Gemini 2.5 Pro model page](https://ai.google.dev/gemini-api/docs/models/gemini-2.5-pro?hl=en), retrieved 2026-07-30.
- [Google Gemini Developer API pricing](https://ai.google.dev/gemini-api/docs/pricing?hl=en), retrieved 2026-07-30.
- [Google Gemini deprecations](https://ai.google.dev/gemini-api/docs/deprecations?hl=en), retrieved 2026-07-30.

## Google stable-ID limitation

Google distinguishes stable, preview, latest, and experimental names. Its documentation says a stable name points to a specific stable model and usually does not change. It separately says a `latest` alias is hot-swapped.

For these three Gemini 2.5 models, however, the public model pages expose only:

- `gemini-2.5-flash-lite`;
- `gemini-2.5-flash`;
- `gemini-2.5-pro`.

They do not expose dated or numbered request-side revision IDs comparable to OpenAI's dated snapshots. The Models API can return model metadata, including a `version` field, but that is observed metadata; the in-scope public model pages do not establish a separate immutable 2.5 revision string that can be selected in the generation request.

This produces two different meanings of pinning:

| Meaning | Gemini 2.5 result |
| --- | --- |
| Pin to Google's documented stable family code | Supported |
| Pin to a publicly documented immutable dated or numbered revision | Not supported by the reviewed first-party documentation |

This second row is an evidence-based inference from the model pages, version-naming guide, Models API reference, and absence of a documented 2.5 revision ID. It is not a claim about an undocumented private provider capability.

Primary authority:

- [Google model version name patterns](https://ai.google.dev/gemini-api/docs/models#model_version_name_patterns), retrieved 2026-07-30.
- [Google Models API reference](https://ai.google.dev/api/models), retrieved 2026-07-30.

## Qualification decision

### Revision-pinnable for a strict sealed comparison

- `gpt-4o-mini-2024-07-18`
- `gpt-4o-2024-08-06`, as a deprecated-family comparator
- `gpt-4o-2024-11-20`, if selected as a separate deprecated-family treatment

These meet the request-side revision requirement, but pinning does not guarantee indefinite availability. They remain subject to a live authenticated availability check, provider response lineage, account entitlement, and the exact price authority captured at execution time. GPT-4o's catalog deprecation means it should not be promoted as a new long-lived production default merely because its snapshots can be pinned.

### Documented but not strictly revision-pinnable

- `gemini-2.5-flash-lite`
- `gemini-2.5-flash`
- `gemini-2.5-pro`

If the fail-closed contract requires an immutable request-side model revision, these three actions must not be accepted as sealed efficacy evidence. The available remedies are:

1. obtain provider authority for an immutable request-side revision;
2. formally approve a narrower definition in which Google's stable ID plus captured Models API metadata is sufficient;
3. use the models only for non-sealed exploratory evidence and keep them out of promotion decisions.

Any exception must be explicit. The harness must not silently downgrade the revision requirement.

## Required evidence for the live gate

For every model action, retain:

1. provider and provider endpoint;
2. exact requested model string;
3. exact model identifier or version returned by the provider, when supplied;
4. request and response timestamps;
5. account region and processing mode without recording credentials;
6. prompt input-token count, cached-token count, output-token count, and thinking-token count where available;
7. output-budget setting and actual output usage;
8. price-source URL, retrieval timestamp, currency, modality, context tier, and storage/tool surcharges;
9. provider request ID and sanitized raw-response lineage;
10. deprecation status and announced shutdown date;
11. explicit pass or fail of the revision-pinning requirement.

## Source register

All sources are first-party and were retrieved on 2026-07-30.

| Authority | URL | Use |
| --- | --- | --- |
| OpenAI GPT-4o mini | https://developers.openai.com/api/docs/models/gpt-4o-mini | ID, default snapshot, endpoints, limits, pricing |
| OpenAI GPT-4o | https://developers.openai.com/api/docs/models/gpt-4o | ID, default snapshot, endpoints, limits, pricing |
| OpenAI all-model catalog | https://developers.openai.com/api/docs/models/all | Family-level deprecation label |
| OpenAI pricing | https://developers.openai.com/api/docs/pricing | Standard, Batch, and Priority token prices |
| OpenAI prompt caching | https://developers.openai.com/api/docs/guides/prompt-caching | Cached-token eligibility and usage reporting |
| OpenAI deprecations | https://developers.openai.com/api/docs/deprecations | Current deprecation and shutdown authority |
| Google Gemini model catalog and version patterns | https://ai.google.dev/gemini-api/docs/models | Model availability and stable/latest semantics |
| Google Gemini 2.5 Flash-Lite | https://ai.google.dev/gemini-api/docs/models/gemini-2.5-flash-lite?hl=en | Model code, stable status, limits, capabilities |
| Google Gemini 2.5 Flash | https://ai.google.dev/gemini-api/docs/models/gemini-2.5-flash?hl=en | Model code, stable status, limits, capabilities |
| Google Gemini 2.5 Pro | https://ai.google.dev/gemini-api/docs/models/gemini-2.5-pro?hl=en | Model code, stable status, limits, capabilities |
| Google Gemini pricing | https://ai.google.dev/gemini-api/docs/pricing?hl=en | Standard paid-tier token and cache pricing |
| Google context caching | https://ai.google.dev/gemini-api/docs/caching?hl=en | Implicit-cache behavior, thresholds, and reported cache hits |
| Google Gemini deprecations | https://ai.google.dev/gemini-api/docs/deprecations?hl=en | Announced earliest shutdown dates and replacements |
| Google GenerateContent API | https://ai.google.dev/api/generate-content | Generation wire-path format |
| Google Models API | https://ai.google.dev/api/models | Model resource naming and observable version metadata |
