# Changelog

All notable changes to this project will be documented in this file.

## [9.0.0] - 2026-11-02

### 🐛 Bug Fixes

- Fix paths for the database building documentation
- *(core)* [**breaking**] Fix consensus classification and I/O bugs, add test suite
- [**breaking**] Keep leading lineage ranks, cap threads and replace panics with errors

### 🚜 Refactor

- Remove tha main changelog over the individual files into workspace crates

### 📚 Documentation

- *(blutils-book)* Initialize the blutils book
- *(docs-dependencies)* Create an additional file to install needed dependencies for the qc pipeline run
- Include the tabular parser in blutils documentation and remove 4th step
- Create the documentation for the blutils online viewer
- Update the main readme to redirect users to the book
- Refresh online viewer screenshots
- Fix jsonl log format label in the README
- Describe the 2025 NCBI taxdump rank changes

### 🧪 Testing

- *(cli)* Accept mock lineages that stop above species in smoke tests

### ⚙️ Miscellaneous Tasks

- Run tests and clippy on push and pull requests

## [8.3.1] - 2025-05-21

### 🚀 Features

- *(yaml output)* Implements the yaml output format parsing as an aditional option for parsing blutils results

### ⚙️ Miscellaneous Tasks

- Release

## [8.3.0] - 2025-05-20

### 🚀 Features

- *(tracing)* Improve the traccing instruments to disable log to the stdout

### ⚙️ Miscellaneous Tasks

- *(cliff)* Add cliff to the project to replace the standard pythons commitizen
- Remove python commitizen artifacts
- Release
- *(cliff)* Add individual changelog files to the workspace project
- Release
- Release

## [8.2.0] - 2024-03-21

### 🚀 Features

- Implements the option to set custom taxon cutoffs allowing to extends default gene divergence cutoffs for any gene

### 💼 Other

- 8.1.5 → 8.2.0 [skip-ci]

## [8.1.5] - 2024-03-20

### 🐛 Bug Fixes

- Rename the default kraken files

### 💼 Other

- 8.1.4 → 8.1.5 [skip-ci]

## [8.1.4] - 2024-03-19

### 🐛 Bug Fixes

- Fix the output file writing after the consensus generation

### 💼 Other

- 8.1.3 → 8.1.4 [skip-ci]

### ⚙️ Miscellaneous Tasks

- Create a base script to publish all packages

## [8.1.3] - 2024-03-19

### 🐛 Bug Fixes

- Fix error during creation of the blutils output file if it not exists

### 💼 Other

- 8.1.2 → 8.1.3 [skip-ci]

## [8.1.2] - 2024-03-14

### 🐛 Bug Fixes

- *(create-kraken2-db)* Allow deletion of files and dirs on check the path for the ouput of kraken database

### 💼 Other

- 8.1.1 → 8.1.2 [skip-ci]

## [8.1.1] - 2024-03-14

### 🐛 Bug Fixes

- *(jsonl-output)* Include line breaks on write jsonl file

### 💼 Other

- 8.1.0 → 8.1.1 [skip-ci]

## [8.1.0] - 2024-03-14

### 🚀 Features

- Implements the kraken database custom source files generator from a blast database

### 💼 Other

- 8.0.1 → 8.1.0 [skip-ci]

### 🚜 Refactor

- Renament blutils database generator use-case

## [8.0.1] - 2024-03-14

### 🐛 Bug Fixes

- *(write-to-stdout)* Fix cli methods related to read and write data from stdin and stdout

### 💼 Other

- 8.0.0 → 8.0.1 [skip-ci]

## [8.0.0] - 2024-03-14

### 🚀 Features

- [**breaking**] Replace all positional arguments of the cli by nominal arguments with exception of the query sequences
- Allow stdin and stdout redirects

### 🐛 Bug Fixes

- *(tabular-result)* Fix the tabular columns order

### 💼 Other

- 7.2.0 → 8.0.0 [skip-ci]

### 🧪 Testing

- Add test data from zymo sra validation study

## [7.2.0] - 2024-03-03

### 🚀 Features

- Implements a cli command to convert the default blutils json result into a tabular result

### 💼 Other

- 7.1.3 → 7.2.0 [skip-ci]

## [7.1.3] - 2024-03-03

### 🐛 Bug Fixes

- Improve the write or append to file auxiliary function to allow write to file with it oppened
- Fix the consensus generation to replace the sequence hash by the original id during the blutils database generation

### 💼 Other

- 7.1.2 → 7.1.3 [skip-ci]

## [7.1.2] - 2024-02-21

### 🐛 Bug Fixes

- *(gh:issue4)* Include exceptions to enable the validation of chunked blast databases

### 💼 Other

- 7.1.1 → 7.1.2 [skip-ci]

## [7.1.1] - 2024-02-21

### 🐛 Bug Fixes

- *(gh:issue3)* The qiime default header was included into the generated qiime database

### 💼 Other

- 7.1.0 → 7.1.1 [skip-ci]

## [7.1.0] - 2024-02-21

### 🚀 Features

- Implements a converter to build the qiime database from the blutils one

### 💼 Other

- 7.0.5 → 7.1.0 [skip-ci]

### 🧪 Testing

- Remove test artifacts from tracked files

## [7.0.5] - 2024-02-19

### 🐛 Bug Fixes

- Wip - fix the database generation to allow low memory machines to generate databases
- Fix the consensus generation to include additional information from the blast results

### 💼 Other

- 7.0.4 → 7.0.5 [skip-ci]

### 🚜 Refactor

- Refacrore blat command to split commands from execution parts

## [7.0.4] - 2024-02-11

### 🐛 Bug Fixes

- *(build-db)* Fix bug on generate reference database which missing names data on load inicial taxdump database

### 💼 Other

- 7.0.3 → 7.0.4 [skip-ci]

### 📚 Documentation

- Remove unwanted test file
- Upgrade readme file and example mock files for tests

## [7.0.3] - 2024-02-11

### 🐛 Bug Fixes

- *(consensus-generation-use-case)* Fix the consensus generation issue that leads our algorithm to select a clade below the maximum expexted lca ideitity

### 💼 Other

- 7.0.2 → 7.0.3 [skip-ci]

## [7.0.2] - 2024-02-07

### 🐛 Bug Fixes

- Fix the lca filtration process
- Fix the multi-taxa filtration process

### 💼 Other

- 7.0.1 → 7.0.2 [skip-ci]

### 📚 Documentation

- Replace the old ordered file ot results to mirror the sorted order of the current result

## [7.0.1] - 2024-02-06

### 🐛 Bug Fixes

- *(consensus-generation)* Fix the generation of the consensus identities that stop the search for subjects early without evaluate the full results

### 💼 Other

- 7.0.0 → 7.0.1 [skip-ci]

## [7.0.0] - 2024-02-05

### 🚀 Features

- Finish the implementation of the consensus generator for the blast command

### 🐛 Bug Fixes

- Set the non-linnaean rank other as untagged
- Replace the taxid by identifier in taxonomy parsing to allow input non taxid values

### 💼 Other

- 6.3.3 → 7.0.0 [skip-ci]

### 🚜 Refactor

- Refacores the blast run with consensus use-case to allow search for the code at the file level
- Refactors building database related use-cases and dtos related to blast results generation
- Move the blast build use-cases to a dedicated module

## [6.3.3] - 2024-02-02

### 🐛 Bug Fixes

- Include the main workspace package in cz tracked files
- Include the taxid when taxnames are not available allowing users to check identities
- Fix the database generation to includ eoptions to remove non-linnaean ranks to kept the database more consistene

### 💼 Other

- Remove the workspace variable adapters cargo file
- Remove workspace notation from cli cargo file
- 6.3.2 → 6.3.3 [skip-ci]

## [6.3.2] - 2024-01-31

### 🐛 Bug Fixes

- Fix dependencies to local package to allow publish in crate
- Fix the workspace dependency from local packages

### 💼 Other

- Fix versions from workspace package rust
- 6.3.1 → 6.3.2 [skip-ci]

## [6.3.1] - 2024-01-31

### 🐛 Bug Fixes

- Fix the database building command

### 💼 Other

- 6.3.0 → 6.3.1 [skip-ci]

## [6.3.0] - 2024-01-29

### 🚀 Features

- Replace the default log from tracing and refactores the workspace to share dependencies
- Wip - implements the database builder to create the initial database for blast runst

### 💼 Other

- 6.2.0 → 6.3.0 [skip-ci]

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
- Include a print function to output the user arguments on cli execution

### 💼 Other

- 5.0.1 → 6.0.0 [skip-ci]

### 📚 Documentation

- Upgrade the default output of blutils

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

### 📚 Documentation

- Upgrade changelog to include new changes

## [3.0.1] - 2023-06-25

### 💼 Other

- Downlgrade version of colored package and upgrade changelog messages
- 3.0.0 → 3.0.1 [skip-ci]

### 🚜 Refactor

- Split the project into sub-crates to better manage dependencies
- Remove unused dependencies from packages crates

### ⚙️ Miscellaneous Tasks

- Include commitizen config file

## [3.0.0] - 2023-06-25

### 🚀 Features

- [**breaking**] Passthrough all blast builder arguments to the adapter executor

### 💼 Other

- 2.0.0 → 3.0.0 [skip-ci]

## [2.0.0] - 2023-06-25

### 🚀 Features

- Initial-commit
- Wip - create the project base
- Create a method to parse taxonomy literals as a internal object
- Wip - start implemetation of the consensus check for the blast results
- Split the parallel blast execution from the main use-case
- Update consensus results to include a more informative results object
- Upgrade the cli port that execute the blast with consensus
- Implements the identity based filtration of the taxon rank
- Implements the consensus check for multiple query results
- Implements the system check and upgrade logs to turn it better for human users

### 🐛 Bug Fixes

- Remove old rustfmt file
- Fix the sort order of imports of the identity based filtration
- Fix image paths on readme file
- Change type of bit-score from int64 to float64

### 💼 Other

- Update crates categories
- Upgrade crate version
- Upgrade package version from 0.3.0 to 0.3.1
- Upgrade application dependencies
- 1.0.0 → 2.0.0 [skip-ci]

### 🚜 Refactor

- Review the access control modifiers of the application

### 📚 Documentation

- Create a simple readme file and a todo list
- Include key metadata into the project cargo file
- Create diagrams for the parallel blast and the consensus generation steps
- [**breaking**] Upgrade the readme and cli port documentations after publishing on crates

<!-- generated by git-cliff -->
