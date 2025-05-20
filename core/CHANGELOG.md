# Changelog

All notable changes to this project will be documented in this file.

## [8.3.0] - 2025-05-20

### 🚀 Features

- Replace the default log from tracing and refactores the workspace to share dependencies
- Wip - implements the database builder to create the initial database for blast runst
- Finish the implementation of the consensus generator for the blast command
- Implements a converter to build the qiime database from the blutils one
- Implements a cli command to convert the default blutils json result into a tabular result
- [**breaking**] Replace all positional arguments of the cli by nominal arguments with exception of the query sequences
- Allow stdin and stdout redirects
- Implements the kraken database custom source files generator from a blast database
- Implements the option to set custom taxon cutoffs allowing to extends default gene divergence cutoffs for any gene

### 🐛 Bug Fixes

- Fix the database building command
- Include the taxid when taxnames are not available allowing users to check identities
- Fix the database generation to includ eoptions to remove non-linnaean ranks to kept the database more consistene
- Set the non-linnaean rank other as untagged
- Replace the taxid by identifier in taxonomy parsing to allow input non taxid values
- *(consensus-generation)* Fix the generation of the consensus identities that stop the search for subjects early without evaluate the full results
- Fix the lca filtration process
- Fix the multi-taxa filtration process
- *(consensus-generation-use-case)* Fix the consensus generation issue that leads our algorithm to select a clade below the maximum expexted lca ideitity
- *(build-db)* Fix bug on generate reference database which missing names data on load inicial taxdump database
- Wip - fix the database generation to allow low memory machines to generate databases
- Fix the consensus generation to include additional information from the blast results
- *(gh:issue3)* The qiime default header was included into the generated qiime database
- *(gh:issue4)* Include exceptions to enable the validation of chunked blast databases
- Improve the write or append to file auxiliary function to allow write to file with it oppened
- Fix the consensus generation to replace the sequence hash by the original id during the blutils database generation
- *(tabular-result)* Fix the tabular columns order
- *(write-to-stdout)* Fix cli methods related to read and write data from stdin and stdout
- *(jsonl-output)* Include line breaks on write jsonl file
- *(create-kraken2-db)* Allow deletion of files and dirs on check the path for the ouput of kraken database
- Fix error during creation of the blutils output file if it not exists
- Fix the output file writing after the consensus generation
- Rename the default kraken files

### 💼 Other

- Fix versions from workspace package rust

### 🚜 Refactor

- Refacores the blast run with consensus use-case to allow search for the code at the file level
- Refactors building database related use-cases and dtos related to blast results generation
- Move the blast build use-cases to a dedicated module
- Refacrore blat command to split commands from execution parts
- Renament blutils database generator use-case

## [6.2.0] - 2023-09-12

### 🚀 Features

- Wip - start implementation of the database generator parts

### 🐛 Bug Fixes

- Comment ref-database functionalities from the project temporary

### 💼 Other

- 6.1.3 → 6.2.0 [skip-ci]

## [6.1.3] - 2023-07-07

### 💼 Other

- 6.1.2 → 6.1.3 [skip-ci]

### 🚜 Refactor

- Move the blast run to a dedicated submodule

## [6.1.2] - 2023-07-05

### 🐛 Bug Fixes

- Include kingdom as valida taxonomic rank

### 💼 Other

- 6.1.1 → 6.1.2 [skip-ci]

## [6.1.1] - 2023-07-05

### 🐛 Bug Fixes

- Include in debug message when an error is found on parse taxonomy

### 💼 Other

- 6.1.0 → 6.1.1 [skip-ci]

## [6.1.0] - 2023-06-30

### 🚀 Features

- Upgrade cli port to include blast arguments as parameters

### 💼 Other

- 6.0.0 → 6.1.0 [skip-ci]

## [6.0.0] - 2023-06-30

### 🚀 Features

- Include options to control the word-size during blast executions
- [**breaking**] Include taxonomy in results and review the taxid and rank information delivered to the output object

### 💼 Other

- 5.0.1 → 6.0.0 [skip-ci]

### 🎨 Styling

- Increase verbosity of the parallel blast use-case execution

## [5.0.1] - 2023-06-25

### 💼 Other

- 5.0.0 → 5.0.1 [skip-ci]

### 🚜 Refactor

- *(filter_rank_by_identity)* Remove unwanted print from filter_rank_by_identity use-case

## [5.0.0] - 2023-06-25

### 🐛 Bug Fixes

- [**breaking**] Fix bug on filter taxonomies by rank

### 💼 Other

- 4.0.1 → 5.0.0 [skip-ci]

## [4.0.1] - 2023-06-25

### 🐛 Bug Fixes

- Remove the min_consensus argument which is not used

### 💼 Other

- 4.0.0 → 4.0.1 [skip-ci]

### 🚜 Refactor

- Rename blastn execution entity and adapter to match the target of the execution which is the blastn

## [4.0.0] - 2023-06-25

### 💼 Other

- [**breaking**] Fix dependencies relationships between packages that compose the blutils package
- 3.0.1 → 4.0.0 [skip-ci]

## [3.0.1] - 2023-06-25

### 💼 Other

- 3.0.0 → 3.0.1 [skip-ci]

### 🚜 Refactor

- Split the project into sub-crates to better manage dependencies
- Remove unused dependencies from packages crates

<!-- generated by git-cliff -->
