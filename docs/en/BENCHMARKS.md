# Evaluation and limits

*[Versione italiana](../it/BENCHMARKS.md)*

This distribution does not include the training or evaluation dataset, the
original documents, the reference transcriptions or any participant
identifier. The screenshots use synthetic content and are not an OCR benchmark.

## Method

Internal evaluation compares the base model with the fine-tune on Italian
pages, looking at character and word error rates (CER and WER), incomplete
pages, repetitions and end-to-end pipeline latency. Page preparation, backend,
quantisation and decoding strategy all affect the outcome and must be reported
alongside any figure.

The final held-out set was consulted several times during development: it is
not an untouched independent test. Internal figures should therefore be read as
descriptive. No leaderboard and no universal accuracy percentage is published.

The technical report [*Teaching a Vision Model When to Stop*](../assets/ITA-OCR-report-tecnico.pdf)
documents the split protocol, the termination collapse observed after
fine-tuning and the inference cascade that compensates for it.

## Measured results

The figures below come from the [technical report](../assets/ITA-OCR-report-tecnico.pdf)
and compare the base model with the shipped system. Every row carries the set it
was measured on, because that is what gives the percentage meaning. The noise
floor of the measurements is 0.3 points: the margins reported here are far wider.

| Set | Pages | Character error | Word error |
| --- | --- | --- | --- |
| Holdout, writers disjoint from training | 164 | 30.88 → 25.71 (−16.7%) | 54.68 → 45.54 (−16.7%) |
| Sealed benchmark, readable part | 67 | 29.28 → 18.78 (−35.9%) | 56.74 → 38.99 (−31.3%) |
| Cohort | 136 | −8.8% | −20.4% |
| Subset labelled easy a priori | 61 | 12.06 → 11.28 (−6.5%) | 36.97 → 30.67 (−17.0%) |

The holdout uses capped character error, the only fair statistic where pages are
lost; on the other sets neither model loses a page, so raw and capped coincide.
The benchmark row excludes a single writer at the edge of legibility, who alone
moves the aggregate more than any difference between models.

On the easy subset, page by page, the shipped system wins on 43, loses on 17 and
ties on 1: the aggregate advantage is real, and it is not uniform.

### Lost pages and the cascade

| Configuration | Pages lost out of 48 |
| --- | --- |
| Greedy decoding, no mitigation | 24 |
| Frequency penalty 0.35 + presence 0.20 | 5 |
| Presence penalty 0.20 alone | 22 |
| DRY sampling | 22 |

The cascade costs less than not having it: on the holdout a full cascaded run
takes less time than a single plain greedy pass over the same pages, because
runaway pages are cut at around 550 tokens instead of running to the 4,096 token
ceiling, and the retry that follows is short.

### Quantisation and CPU

The shipped weights are Q8_0 for both towers. Against bf16 they use about 30%
less video memory, decode about 35% faster and are paired-identical in quality.
Q5_K_M and Q4_K_M show a small but systematic degradation.

On the CPU the quality does not change, the time does. On the test machine a
page goes from a little over a second and a half on the GPU to roughly fifteen
seconds on the CPU, and most of that time goes into encoding the image rather
than generating the text. **The absolute values depend on the machine** (processor,
graphics card, drivers, page complexity) and should be read as a
ratio, not as guaranteed performance. On the CPU, quantisation buys memory
rather than latency: around 2.4 GB resident with Q8_0. The lever for latency
would be the resolution of the page, not the precision of the weights.

## Package checks

The Windows build was exercised on an AMD Radeon RX 7900 XT with Vulkan. That
verifies the pipeline can complete a document on that configuration; it
certifies neither every GPU nor the quality of every transcription. The UI
suite uses a simulated bridge and checks the application's interactions.

## Known limits

- Difficult handwriting, irregular layout, formulas and tables can cause
  omissions, substitutions, repetitions or invented content.
- Dictionary-based correction can introduce errors: substitutions stay
  highlighted and the result must be compared with the original.
- Small variations in GPU decoding can produce different results.
- Recognition on CPU can be considerably slower.
- Results obtained with private lexical resources do not transfer automatically
  to the public package, which uses public dictionaries only.

For a public reproduction, use synthetic documents or data you are allowed to
distribute, and document the environment, the parameters and the measurement
criteria.
