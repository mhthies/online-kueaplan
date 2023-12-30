#!/bin/bash
cd "$(dirname "$0")"
openapi-generator generate -g html2 -i openapi.json -o build
