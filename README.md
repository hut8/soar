# SOAR

Situational Overview and Aircraft Record

## Overview

* Connects to aprs.glidernet.org:14580 (for filter) or aprs.glidernet.org:10152 (full feed)
* Performs handshake and filter configuration.
* Sends a keep-alive consisting of an octothorpe followed by information about the receiver periodically.
*

## Development

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## Device database

[DDB on Glidernet.org](https://ddb.glidernet.org/download/?j=1)

## Questions

- https://github.com/glidernet/ogn-ddb/blob/master/ogn-ddb-schema-1.0.0.json - what are F, I, and O?
