# Privacy and data

*[Versione italiana](PRIVACY.md)*

## Where processing happens

OCR inference runs locally through llama.cpp. The engine server listens on the
loopback interface `127.0.0.1` only, not on every network interface. Documents
never need to be uploaded to a remote service to be transcribed, and once
installed the application works offline.

The app contains no telemetry, sends no usage statistics and requires no
account. The network is used only to download the installer and the model
weights, once, and for web pages you deliberately open from the interface.

## What stays on your machine

The app keeps its history, settings and logs in the user profile
(`%APPDATA%\ocr-ita-desktop` on Windows). These may contain document text,
file names and paths chosen by the user.
**Uninstalling does not remove the history automatically:** if it holds
personal data, delete it separately.

## ITA-OCR and the GDPR

Local processing narrows the scope of processing in concrete ways:

- No external processor is involved in the OCR step.
- No data is transferred to third countries.
- Documents never leave the device of whoever holds them.
- There is no server-side copy to request erasure or portability from.

**This is not a certificate of compliance.** ITA-OCR is a tool: compliance
remains with the data controller and depends on the legal basis, the privacy
notice, data minimisation, retention periods, device security, disk
encryption, backup handling and deletion of the local history. Anyone
processing special categories of data, or other people's data in a
professional capacity, should assess their own situation independently.

The project is not a service: there is no controller receiving your documents,
because nobody receives them.

## Website and external services

The project website is static: it offers no upload and no in-browser
transcription. It carries no analytics and no data-collection forms. The
hosting provider may collect technical HTTP request logs. Links to GitHub and
Hugging Face open external services governed by their own terms and notices.

## Dataset and model

The training and evaluation dataset stays private. No original documents,
reference transcriptions, manifests, personal identifiers or lexical resources
derived from the private corpus are distributed. Public examples and images are
synthetic.

A model can still make mistakes or memorise parts of its training data: no
guarantee of absence of memorisation is claimed.

## Before sharing anything

Before attaching logs, screenshots or documents to an issue, replace the
content with a synthetic example and strip metadata and personal paths.

Questions about this document: **giorgio.ottoboni@proton.me**.
