# Create Blutils Database

Blutils is a blastn wrapper, so it needs a blast database together the NCBI
taxonomies to build the Blutils database.

Here we will see how to create the Blutils database based on NCBI taxonomy dump
file (available in the NCBI FTP
[here](https://ftp.ncbi.nlm.nih.gov/pub/taxonomy/new_taxdump/new_taxdump.tar.gz)) and the official 16S database
(available in the NCBI FTP
[here](https://ftp.ncbi.nlm.nih.gov/blast/db/16S_ribosomal_RNA.tar.gz)).

## Download and decompress the databases

First, create a directory to store the databases:

```bash
mkdir -p blutils_db/{new_taxdump,blast_db,blutils_db}
```

Then, download the NCBI taxonomy dump file and the 16S database:

```bash
curl ftp://ftp.ncbi.nlm.nih.gov/pub/taxonomy/new_taxdump/new_taxdump.tar.gz \
    --output blutils_db/new_taxdump/new_taxdump.tar.gz

curl ftp://ftp.ncbi.nlm.nih.gov/pub/taxonomy/new_taxdump/new_taxdump.tar.gz.md5 \
    --output blutils_db/new_taxdump/new_taxdump.tar.gz.md5

curl ftp://ftp.ncbi.nlm.nih.gov/blast/db/16S_ribosomal_RNA.tar.gz \
    --output blutils_db/blast_db/16S_ribosomal_RNA.tar.gz

curl ftp://ftp.ncbi.nlm.nih.gov/blast/db/16S_ribosomal_RNA.tar.gz.md5 \
    --output blutils_db/blast_db/16S_ribosomal_RNA.tar.gz.md5
```

Verify the integrity of the downloaded files:

```bash
cd blutils_db/new_taxdump
md5sum -c new_taxdump.tar.gz.md5

cd ../blast_db
md5sum -c 16S_ribosomal_RNA.tar.gz.md5

cd ../..
```

If the files are corrupted, download them again.

Now, decompress the 16S database:

```bash
tar -xzf blutils_db/blast_db/16S_ribosomal_RNA.tar.gz -C blutils_db/blast_db
```

The output directory should contain the following files:

```bash
total 278M
-rw-rw-r-- 1 sgelias sgelias 1.4M May 24 06:36 16S_ribosomal_RNA.ndb
-rw-rw-r-- 1 sgelias sgelias 4.2M May 24 06:36 16S_ribosomal_RNA.nhr
-rw-rw-r-- 1 sgelias sgelias 319K May 24 06:36 16S_ribosomal_RNA.nin
-rw-rw-r-- 1 sgelias sgelias 216K May 24 06:36 16S_ribosomal_RNA.nnd
-rw-rw-r-- 1 sgelias sgelias  908 May 24 06:36 16S_ribosomal_RNA.nni
-rw-rw-r-- 1 sgelias sgelias 107K May 24 06:36 16S_ribosomal_RNA.nog
-rw-rw-r-- 1 sgelias sgelias 536K May 24 06:36 16S_ribosomal_RNA.nos
-rw-rw-r-- 1 sgelias sgelias 320K May 24 06:36 16S_ribosomal_RNA.not
-rw-rw-r-- 1 sgelias sgelias 9.6M May 24 06:36 16S_ribosomal_RNA.nsq
-rw-rw-r-- 1 sgelias sgelias 664K May 24 06:36 16S_ribosomal_RNA.ntf
-rw-rw-r-- 1 sgelias sgelias 191K May 24 06:36 16S_ribosomal_RNA.nto
-rw-rw-r-- 1 sgelias sgelias 162M May 24 06:36 taxdb.btd
-rw-rw-r-- 1 sgelias sgelias  18M May 24 06:36 taxdb.bti
-rw-rw-r-- 1 sgelias sgelias  82M May 24 06:36 taxonomy4blast.sqlite3
```

And decompress the taxdump:

```bash
tar -xzf blutils_db/new_taxdump/new_taxdump.tar.gz -C blutils_db/new_taxdump
```

The output directory should contain the following files:

```bash
total 1.9G
-rw-r--r-- 1 sgelias sgelias  19M May 25 19:38 citations.dmp
-rw-r--r-- 1 sgelias sgelias 6.2M May 25 19:36 delnodes.dmp
-rw-r--r-- 1 sgelias sgelias  452 May 25 19:31 division.dmp
-rw-r--r-- 1 sgelias sgelias  51K May 25 19:38 excludedfromtype.dmp
-rw-r--r-- 1 sgelias sgelias 714M May 25 19:36 fullnamelineage.dmp
-rw-r--r-- 1 sgelias sgelias 4.9K May 25 19:31 gencode.dmp
-rw-r--r-- 1 sgelias sgelias 5.6M May 25 19:38 host.dmp
-rw-r--r-- 1 sgelias sgelias 4.6M May 25 19:36 images.dmp
-rw-r--r-- 1 sgelias sgelias 1.7M May 25 19:36 merged.dmp
-rw-r--r-- 1 sgelias sgelias 235M May 25 19:38 names.dmp
-rw-r--r-- 1 sgelias sgelias 240M May 25 19:38 nodes.dmp
-rw-r--r-- 1 sgelias sgelias 346M May 25 19:36 rankedlineage.dmp
-rw-r--r-- 1 sgelias sgelias 303M May 25 19:36 taxidlineage.dmp
-rw-r--r-- 1 sgelias sgelias  30M May 25 19:38 typematerial.dmp
-rw-r--r-- 1 sgelias sgelias 3.0K May 25 19:36 typeoftype.dmp
```

## Build the Blutils database

Now we can build the Blutils database. First, we need to check the available
options for the `blu build-db` command:

```bash
blu build-db blu --help
```

Note the subpath `blu` after the `blu build-db` command. This is needed because
blutils contains additional commands used to generate databases for Kraken2 and
QIIME2 (such functionalities will not be covered in this tutorial). See `blu
build-db --help` for more information.

Now, the output for the `blu build-db blu --help` command should be close to:

```bash
Build the Blutils database

Usage: blu build-db blu [OPTIONS] <BLAST_DATABASE_PATH> <TAXDUMP_DIRECTORY_PATH> <OUTPUT_FILE_PATH>

Arguments:
  <BLAST_DATABASE_PATH>
          The path to the blast database
          
          The path to the blast database that will be used to build the consensus taxonomy. The blast database should be a nucleotide database. The database should be created using the makeblastdb command from the blast package.

  <TAXDUMP_DIRECTORY_PATH>
          The path to the taxdump directory
          
          The path to the taxdump directory that contains the NCBI taxonomy database. The taxdump directory should be downloaded from the NCBI taxonomy database. The taxdump should be downloaded from the NCBI and uncompressed.

  <OUTPUT_FILE_PATH>
          The path where the output file will be saved
          
          The output file is a JSON file that contains the taxonomies database.

Options:
  -d, --drop-non-linnaean-taxonomies
          Drop non Linnaean taxonomies
          
          If this option is set, the non Linnaean taxonomies will be dropped from the taxonomy building process. The non Linnaean taxonomies are the ones that are not part of the Linnaean taxonomy system. The default value is false.

  -s, --skip-taxid <SKIP_TAXID>
          Specify taxids to be skipped Example: --skip-taxid 131567
          
          The specified taxid will be skipped in the taxonomy building process. It should be used to skip multiple taxids. Example: --skip-taxid 131567 --skip-taxid 2

  -r, --replace-rank <REPLACE_RANK>
          Replace a rank by another Example: --replace-rank 'superkingdom=d'. It is common to use this option to replace the superkingdom rank by domain in bacterial taxonomy.
          
          Multiple ranks can be replaced by using the option multiple times. Example: --replace-rank 'superkingdom=d' --replace-rank 'clade=cl'

  -h, --help
          Print help (see a summary with '-h')
```

Then, we can build the database using the `blu build-db blu` command:

```bash
blu build-db blu \
    blutils_db/blast_db/16S_ribosomal_RNA \
    blutils_db/new_taxdump \
    blutils_db/blutils_db/16S_ribosomal_RNA
```

`Blutils` will create two files at the `blutils_db/blutils_db` directory:

- `16S_ribosomal_RNA.blutils.json`: a JSON file containing the taxonomies
  database.
- `16S_ribosomal_RNA.non-mapped.tsv`: a TSV file containing the non-mapped
  sequences.

The `16S_ribosomal_RNA.blutils.json` file contains the a JSON database like
the following:

```json
{
  "blutilsVersion": "8.3.0",
  "ignoreTaxids": null,
  "replaceRank": null,
  "dropNonLinnaeanTaxonomies": false,
  "sourceDatabase": "/absolute/path/to/blutils_db/blast_db/16S_ribosomal_RNA",
  "taxonomies": [
    {
      "taxid": 259354,
      "rank": "s",
      "numericLineage": "no-rank__131567;superkingdom__2;p__200940;c__3024418;o__213118;f__3031627;g__218207;s__259354",
      "textLineage": "no-rank__cellular-organisms;superkingdom__bacteria;p__thermodesulfobacteriota;c__desulfobacteria;o__desulfobacterales;f__desulfatibacillaceae;g__desulfatibacillum;s__desulfatibacillum-alkenivorans",
      "accessions": [
        {
          "accession": "NR_025795.1",
          "oid": "1878"
        }
      ]
    },
    {
      "taxid": 1006576,
      "rank": "s",
      "numericLineage": "no-rank__131567;superkingdom__2;p__200918;c__188708;o__1643947;f__1643949;g__1511648;s__1006576",
      "textLineage": "no-rank__cellular-organisms;superkingdom__bacteria;p__thermotogota;c__thermotogae;o__petrotogales;f__petrotogaceae;g__defluviitoga;s__defluviitoga-tunisiensis",
      "accessions": [
        {
          "accession": "NR_122085.1",
          "oid": "13670"
        }
      ]
    },
    ...
  ]
}
```

The `taxonomies` field contains the taxonomies database. Each taxon is
represented by a `taxid` field, which is the NCBI TaxID, and a `rank` field,
both the numeric and text lineages (users can choose to use one of them). The
`accessions` field contains the accessions (e.g. `accession` and `oid`) of the
sequences contained in the following taxonomy and the ordinal position of the
sequence in the blast database.

The `16S_ribosomal_RNA.non-mapped.tsv` file contains the sequences present in
the blast database but not in the taxonomies database. In general, this file
should be empty.
