# Run BLAST and generate consensus identities

Based on the Blutils database, we can run BLAST and generate consensus
identities with a single command.

See `blu blastn run-with-consensus --help` for more information. The expected
output for the help command is:

```bash
Run blast and generate consensus identities

Usage: blu blastn run-with-consensus [OPTIONS] --database <DATABASE> --tax-file <TAX_FILE> --blast-out-file <BLAST_OUT_FILE> --taxon <TAXON> --strategy <STRATEGY> [QUERY]

Arguments:
  [QUERY]
          If the value is "-", the STDIN will be used and this command will expect to receive the blutils output from the STDIN
          
          [default: -]

Options:
  -d, --database <DATABASE>
          The reference sequences system file path

  -t, --tax-file <TAX_FILE>
          The taxonomy system file path

      --blast-out-file <BLAST_OUT_FILE>
          The output directory

      --blutils-out-file <BLUTILS_OUT_FILE>
          The output file

      --out-format <OUT_FORMAT>
          The output file format
          
          [default: json]

          Possible values:
          - json:  JSON format
          - jsonl: JSONL format
          - yaml:  Yaml format

      --taxon <TAXON>
          This option checks the higher taxon which the consensus search should be based

          Possible values:
          - fungi:      Fungi cutoff values based on default Internal Transcribed Spacer (ITS) cutoffs
          - bacteria:   Bacteria cutoff values are based on default 16S rRNA cutoffs
          - eukaryotes: Eukaryotes cutoff values are based on fungal Internal Transcribed Spacer (ITS) cutoffs
          - custom:     Custom values must be provided by the user

  -c, --custom-taxon-cutoff-file <CUSTOM_TAXON_CUTOFF_FILE>
          

      --strategy <STRATEGY>
          The strategy to be used

          Possible values:
          - cautious: Select the shortest taxonomic path to find consensus from
          - relaxed:  Select the longest taxonomic path to find consensus from

  -u, --use-taxid
          Use taxid instead of taxonomy
          
          If true, the consensus will be based on the taxid instead of the taxonomy itself.

  -f, --force-overwrite
          Case true, overwrite the output file if exists. Otherwise dispatch an error if the output file exists

  -m, --max-target-seqs <MAX_TARGET_SEQS>
          The max target sequences to be used. Default is 10

  -p, --perc-identity <PERC_IDENTITY>
          The percentage of identity to be used. Default is 80

  -q, --query-cov <QUERY_COV>
          The query coverage to be used. Default is 80

      --strand <STRAND>
          The strand to be used. Default is both
          
          [possible values: both, plus, minus]

  -e, --e-value <E_VALUE>
          The e-value to be used. Default is 0.001

  -w, --word-size <WORD_SIZE>
          The word size to be used. Default is 15

  -h, --help
          Print help (see a summary with '-h')
```

For now, we will download an example SRA data using the
[`fastq-dump`](https://hpc.nih.gov/apps/sratoolkit.html) tool:

```bash
mkdir -p data/SRA
cd data/SRA

fastq-dump --split-files SRR25707968

cd ../..
```

Then, execute a simple quality control pipeline using some common bioinformatics
tools as
[`trimmomatic`](http://www.usadellab.org/cms/uploads/supplementary/Trimmomatic/TrimmomaticManual_V0.32.pdf)
and [`vsearch`](https://pubmed.ncbi.nlm.nih.gov/27781170/). For this, use [this
script](./pipelines/run_qc.sh) as an example. To ensure that the pipeline is
working, install the dependencies using the `install_deps.sh` script in the
[`pipelines`](./pipelines) directory.

```bash
mkdir -p data
cd data
export PIPELINE_URL="https://raw.githubusercontent.com/LepistaBioinformatics/blutils/refs/heads/main/docs/book/pipelines/run_qc.sh"

curl -sSL ${PIPELINE_URL} > run_qc.sh

bash run_qc.sh ./SRA/SRR25707968_1.fastq ./SRA/SRR25707968_2.fastq

cd ..
```

The output directory is `data/qc/SRR25707968` and should contain the
following structure:

```bash
tree data/qc/SRR25707968
├── 01_fastqc
│   ├── lock.lock
│   ├── SRR25707968_1_fastqc.html
│   ├── SRR25707968_1_fastqc.zip
│   ├── SRR25707968_2_fastqc.html
│   └── SRR25707968_2_fastqc.zip
├── 02_trimming
│   ├── lock.lock
│   ├── SRR25707968_1P.fastq.gz
│   ├── SRR25707968_1U.fastq.gz
│   ├── SRR25707968_2P.fastq.gz
│   └── SRR25707968_2U.fastq.gz
├── 03_merge
│   ├── lock.lock
│   ├── SRR25707968.merged.fasta
│   ├── SRR25707968.merge.log
│   ├── SRR25707968.unmerged.forward.fastq
│   └── SRR25707968.unmerged.reverse.fastq
├── 04_dereplicate
│   ├── lock.lock
│   ├── SRR25707968.dereplicated.fasta
│   └── SRR25707968.dereplicate.log
├── 05_denoise
│   ├── lock.lock
│   ├── SRR25707968.denoised.fasta
│   └── SRR25707968.denoise.log
└── 06_chimera
    ├── lock.lock
    ├── SRR25707968.chimera.log
    └── SRR25707968.nonchimeras.fasta

7 directories, 24 files
```

The last contains the chimera free sequences in `SRR25707968.nonchimeras.fasta`,
we will use this file as the query for the BLAST and consensus identity
generation. For this, we will use the `blu blastn run-with-consensus` command
as follows:

```bash
mkdir -p output

cat data/qc/SRR25707968/06_chimera/SRR25707968.nonchimeras.fasta | \
    blu \
    --log-level error \
    --threads 12 \
    blastn run-with-consensus \
    --tax-file blutils_db/blutils_db/16S_ribosomal_RNA.blutils.json \
    --database blutils_db/blast_db/16S_ribosomal_RNA \
    --taxon bacteria \
    --strategy relaxed \
    --blast-out-file output/blast.out.tsv \
    --out-format json \
    --word-size 11 \
    -f > output/blutils.out.json
```

The output file is `output/blutils.out.json` and contains the consensus
identities for the query sequences.
