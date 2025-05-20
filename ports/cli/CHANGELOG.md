# Changelog

All notable changes to this project will be documented in this file.

## [8.3.0] - 2025-05-20

### 🚀 Features

- *(tracing)* Improve the traccing instruments to disable log to the stdout

### ⚙️ Miscellaneous Tasks

- Release
- *(cliff)* Add individual changelog files to the workspace project
- Release

## [8.2.0] - 2024-03-21

### 🚀 Features

- Implements the option to set custom taxon cutoffs allowing to extends default gene divergence cutoffs for any gene

### 💼 Other

- 8.1.5 → 8.2.0 [skip-ci]

## [8.1.5] - 2024-03-20

### 💼 Other

- 8.1.4 → 8.1.5 [skip-ci]

## [8.1.4] - 2024-03-19

### 💼 Other

- 8.1.3 → 8.1.4 [skip-ci]

## [8.1.3] - 2024-03-19

### 💼 Other

- 8.1.2 → 8.1.3 [skip-ci]

## [8.1.2] - 2024-03-14

### 💼 Other

- 8.1.1 → 8.1.2 [skip-ci]

## [8.1.1] - 2024-03-14

### 💼 Other

- 8.1.0 → 8.1.1 [skip-ci]

## [8.1.0] - 2024-03-14

### 🚀 Features

- Implements the kraken database custom source files generator from a blast database

### 💼 Other

- 8.0.1 → 8.1.0 [skip-ci]

## [8.0.1] - 2024-03-14

### 🐛 Bug Fixes

- *(write-to-stdout)* Fix cli methods related to read and write data from stdin and stdout

### 💼 Other

- 8.0.0 → 8.0.1 [skip-ci]

## [8.0.0] - 2024-03-14

### 🚀 Features

- [**breaking**] Replace all positional arguments of the cli by nominal arguments with exception of the query sequences
- Allow stdin and stdout redirects

### 💼 Other

- 7.2.0 → 8.0.0 [skip-ci]

## [7.2.0] - 2024-03-03

### 🚀 Features

- Implements a cli command to convert the default blutils json result into a tabular result

### 💼 Other

- 7.1.3 → 7.2.0 [skip-ci]

## [7.1.3] - 2024-03-03

### 💼 Other

- 7.1.2 → 7.1.3 [skip-ci]

## [7.1.2] - 2024-02-21

### 💼 Other

- 7.1.1 → 7.1.2 [skip-ci]

## [7.1.1] - 2024-02-21

### 💼 Other

- 7.1.0 → 7.1.1 [skip-ci]

## [7.1.0] - 2024-02-21

### 🚀 Features

- Implements a converter to build the qiime database from the blutils one

### 💼 Other

- 7.0.5 → 7.1.0 [skip-ci]

## [7.0.5] - 2024-02-19

### 🐛 Bug Fixes

- Wip - fix the database generation to allow low memory machines to generate databases

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

- Upgrade readme file and example mock files for tests

## [7.0.3] - 2024-02-11

### 💼 Other

- 7.0.2 → 7.0.3 [skip-ci]

## [7.0.2] - 2024-02-07

### 💼 Other

- 7.0.1 → 7.0.2 [skip-ci]

## [7.0.1] - 2024-02-06

### 💼 Other

- 7.0.0 → 7.0.1 [skip-ci]

## [7.0.0] - 2024-02-05

### 🚀 Features

- Finish the implementation of the consensus generator for the blast command

### 💼 Other

- 6.3.3 → 7.0.0 [skip-ci]

### 🚜 Refactor

- Refacores the blast run with consensus use-case to allow search for the code at the file level

## [6.3.3] - 2024-02-02

### 🐛 Bug Fixes

- Fix the database generation to includ eoptions to remove non-linnaean ranks to kept the database more consistene

### 💼 Other

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

## [6.1.2] - 2023-07-05

### 💼 Other

- 6.1.1 → 6.1.2 [skip-ci]

## [6.1.1] - 2023-07-05

### 💼 Other

- 6.1.0 → 6.1.1 [skip-ci]

## [6.1.0] - 2023-06-30

### 🚀 Features

- Upgrade cli port to include blast arguments as parameters

### 💼 Other

- 6.0.0 → 6.1.0 [skip-ci]

## [6.0.0] - 2023-06-30

### 🚀 Features

- Include a print function to output the user arguments on cli execution

### 💼 Other

- 5.0.1 → 6.0.0 [skip-ci]

## [5.0.1] - 2023-06-25

### 💼 Other

- 5.0.0 → 5.0.1 [skip-ci]

## [5.0.0] - 2023-06-25

### 💼 Other

- 4.0.1 → 5.0.0 [skip-ci]

## [4.0.1] - 2023-06-25

### 💼 Other

- 4.0.0 → 4.0.1 [skip-ci]

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
