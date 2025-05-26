#!/bin/bash

# Install trimmomatic

export TRIMMOMATIC_URL="https://github.com/usadellab/Trimmomatic/files/5854859/Trimmomatic-0.39.zip"
export OUTPUT_DIR=/usr/local/bin

curl -sSL ${TRIMMOMATIC_URL} -o /tmp/Trimmomatic-0.39.zip

# Output should be /tmp/trimmomatic/Trimmomatic-0.39/. Assume yes to all
# prompts.
unzip /tmp/Trimmomatic-0.39.zip -d /tmp/trimmomatic

rm /tmp/Trimmomatic-0.39.zip

mv /tmp/trimmomatic/Trimmomatic-0.39/* ${OUTPUT_DIR}

rm -rf /tmp/trimmomatic

# Install vsearch and fastqc

sudo apt install -y vsearch fastqc
