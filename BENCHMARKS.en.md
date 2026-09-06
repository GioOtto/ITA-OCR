# Evaluation and limits

*[Versione italiana](BENCHMARKS.md)*

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

The technical report [*Teaching a Vision Model When to Stop*](docs/assets/ITA-OCR-report-tecnico.pdf)
documents the split protocol, the termination collapse observed after
fine-tuning and the inference cascade that compensates for it.

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
