# Run BLAST and generate consensus taxonomic identities

Based on the Blutils database, we can run BLAST and generate consensus
taxonomic identities with a single command.

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

fastq-dump --split-files SRR20752596

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

bash run_qc.sh ./SRA/SRR20752596_1.fastq ./SRA/SRR20752596_2.fastq

cd ..
```

The output directory is `data/qc/SRR20752596` and should contain the
following structure:

```bash
tree data/qc/SRR20752596
├── 01_fastqc
│   ├── lock.lock
│   ├── SRR20752596_1_fastqc.html
│   ├── SRR20752596_1_fastqc.zip
│   ├── SRR20752596_2_fastqc.html
│   └── SRR20752596_2_fastqc.zip
├── 02_trimming
│   ├── lock.lock
│   ├── SRR20752596_1P.fastq.gz
│   ├── SRR20752596_1U.fastq.gz
│   ├── SRR20752596_2P.fastq.gz
│   └── SRR20752596_2U.fastq.gz
├── 03_merge
│   ├── lock.lock
│   ├── SRR20752596.merged.fasta
│   ├── SRR20752596.merge.log
│   ├── SRR20752596.unmerged.forward.fastq
│   └── SRR20752596.unmerged.reverse.fastq
├── 04_dereplicate
│   ├── lock.lock
│   ├── SRR20752596.dereplicated.fasta
│   └── SRR20752596.dereplicate.log
├── 05_denoise
│   ├── lock.lock
│   ├── SRR20752596.denoised.fasta
│   └── SRR20752596.denoise.log
└── 06_chimera
    ├── lock.lock
    ├── SRR20752596.chimera.log
    └── SRR20752596.nonchimeras.fasta

7 directories, 24 files
```

The last contains the chimera free sequences in `SRR20752596.nonchimeras.fasta`,
we will use this file as the query for the BLAST and consensus identity
generation. For this, we will use the `blu blastn run-with-consensus` command
as follows:

```bash
mkdir -p output

cat data/qc/SRR20752596/06_chimera/SRR20752596.nonchimeras.fasta | \
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
    -f | jq > output/blutils.out.json
```

The output file is `output/blutils.out.json` and contains the consensus
identities for the query sequences as follows:

```bash
{
  "results": [
    {
      "runId": "b324fab1-a9b7-4cdb-b1f3-f8f35b6b22b3",
      "query": "SRR20752596.1002_size_3",
      "taxon": {
        "reachedRank": "species-subgroup",
        "maxAllowedRank": null,
        "identifier": "bacillus-mojavensis-subgroup",
        "percIdentity": 99.356,
        "bitScore": 845.0,
        "taxonomy": "cellular-root__cellular-organisms;d__bacteria;k__bacillati;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus;species-group__bacillus-subtilis-group;species-subgroup__bacillus-mojavensis-subgroup",
        "mutated": false,
        "singleMatch": false,
        "consensusBeans": [
          {
            "rank": "species",
            "identifier": "bacillus-mojavensis",
            "occurrences": 4,
            "taxonomy": "cellular-root__cellular-organisms;d__bacteria;k__bacillati;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus;species-group__bacillus-subtilis-group;species-subgroup__bacillus-mojavensis-subgroup;s__bacillus-mojavensis",
            "accessions": [
              "NR_024693.1",
              "NR_112725.1",
              "NR_116185.1",
              "NR_118290.1"
            ]
          },
          {
            "rank": "species",
            "identifier": "bacillus-halotolerans",
            "occurrences": 3,
            "taxonomy": "cellular-root__cellular-organisms;d__bacteria;k__bacillati;p__bacillota;c__bacilli;o__bacillales;f__bacillaceae;g__bacillus;species-group__bacillus-subtilis-group;species-subgroup__bacillus-mojavensis-subgroup;s__bacillus-halotolerans",
            "accessions": [
              "NR_115063.1",
              "NR_115282.1",
              "NR_115929.1"
            ]
          }
        ]
      }
    }
  ],
  "config": {
    "isConfig": true,
    "runId": "b324fab1-a9b7-4cdb-b1f3-f8f35b6b22b3",
    "blutilsVersion": "8.3.1",
    "subjectReads": "16S_ribosomal_RNA",
    "taxon": "bacteria",
    "outFormat": "6 qseqid saccver staxid pident length mismatch gapopen qstart qend sstart send evalue bitscore",
    "maxTargetSeqs": 10,
    "percIdentity": 80,
    "queryCov": 80,
    "strand": "both",
    "eValue": 0.001,
    "wordSize": 11
  }
}
```

The `results` field contains the consensus taxonomic identities for the query
sequences. Each result contains the following fields:

- `runId`: The run ID.
- `query`: The query sequence.
- `taxon`: The consensus taxonomic identity.

The `taxon` field contains the following fields:

- `reachedRank`: The lowest taxonomic rank that was reached by the consensus
  search.
- `maxAllowedRank`: The highest taxonomic rank that was allowed by the consensus
  search given the `percIdentity` of the query/subject.
- `identifier`: The identifier of the consensus taxonomic identity.
- `percIdentity`: The percentage of identity of the consensus taxonomic identity.
- `bitScore`: The bit score of the consensus taxonomic identity.
- `taxonomy`: The taxonomy of the consensus taxonomic identity. Users can select
  between numeric and string formats.
- `mutated`: Whether the consensus taxonomic identity is the original taxonomy
  or if mycelium was detected needed changes at the taxonomic resolution.
- `singleMatch`: Whether the consensus taxonomic identity is a single match or
  if there are multiple matches.
- `consensusBeans`: The consensus beans used to generate the consensus taxonomic
  identity.

The `consensusBeans` field contains the following fields:

- `rank`: The taxonomic rank.
- `identifier`: The identifier of the taxonomic rank.
- `occurrences`: The number of subject sequences containing a given taxonomy.
- `taxonomy`: The taxonomy of the taxonomic rank.
- `accessions`: The accessions of the subject sequences that were used to
  generate the consensus taxonomic identity.

The `config` field contains the analysis configuration.

## Converting to tabular format

As default `Blutils` outputs the results in JSON format, but users can convert
to a tabular format using the `blu blastn build-tabular` command as follows:

```bash
cat output/blutils.out.json | \
    blu blastn build-tabular | \
    column -t | \
    less -S
```

The output should be similar to the following:

```bash
run-id                                query                    type         rank              identifier                            perc-identity  bit-score  taxonomy                                   >
b324fab1-a9b7-4cdb-b1f3-f8f35b6b22b3  SRR20752596.1002_size_3  consensus    species-subgroup  bacillus-mojavensis-subgroup          99.356         845        cellular-root__cellular-organisms;d__bacter>
b324fab1-a9b7-4cdb-b1f3-f8f35b6b22b3  SRR20752596.1002_size_3  blast-match  species           bacillus-mojavensis                   null           845        cellular-root__cellular-organisms;d__bacter>
b324fab1-a9b7-4cdb-b1f3-f8f35b6b22b3  SRR20752596.1002_size_3  blast-match  species           bacillus-halotolerans                 null           845        cellular-root__cellular-organisms;d__bacter>
```

Each record should contains at last two lines, one for the consensus taxonomic
identity (type: consensus) and one for each blast match (type: blast-match).
