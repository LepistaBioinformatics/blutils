## v7.0.5 (2024-02-18)

### Fix

- fix the consensus generation to include additional information from the blast results
- wip - fix the database generation to allow low memory machines to generate databases

### Refactor

- refacrore blat command to split commands from execution parts

## v7.0.4 (2024-02-11)

### Fix

- **build-db**: fix bug on generate reference database which missing names data on load inicial taxdump database

## v7.0.3 (2024-02-10)

### Fix

- **consensus-generation-use-case**: fix the consensus generation issue that leads our algorithm to select a clade below the maximum expexted lca ideitity

## v7.0.2 (2024-02-06)

### Fix

- fix the multi-taxa filtration process
- fix the lca filtration process

## v7.0.1 (2024-02-05)

### Fix

- **consensus-generation**: fix the generation of the consensus identities that stop the search for subjects early without evaluate the full results

## v7.0.0 (2024-02-04)

### Feat

- finish the implementation of the consensus generator for the blast command

### Fix

- replace the taxid by identifier in taxonomy parsing to allow input non taxid values
- set the non-linnaean rank other as untagged

### Refactor

- move the blast build use-cases to a dedicated module
- refactors building database related use-cases and dtos related to blast results generation
- refacores the blast run with consensus use-case to allow search for the code at the file level

## v6.3.3 (2024-02-01)

### Fix

- fix the database generation to includ eoptions to remove non-linnaean ranks to kept the database more consistene
- include the taxid when taxnames are not available allowing users to check identities
- include the main workspace package in cz tracked files

## v6.3.2 (2024-01-30)

### Fix

- fix the workspace dependency from local packages
- fix dependencies to local package to allow publish in crate

## v6.3.1 (2024-01-30)

### Fix

- fix the database building command

## v6.3.0 (2024-01-28)

### Feat

- wip - implements the database builder to create the initial database for blast runst
- replace the default log from tracing and refactores the workspace to share dependencies

## v6.2.0 (2023-09-12)

### Feat

- wip - start implementation of the database generator parts

### Fix

- comment ref-database functionalities from the project temporary

## v6.1.3 (2023-07-07)

### Refactor

- move the blast run to a dedicated submodule

## v6.1.2 (2023-07-05)

### Fix

- include kingdom as valida taxonomic rank

## v6.1.1 (2023-07-05)

### Fix

- include in debug message when an error is found on parse taxonomy

## v6.1.0 (2023-06-30)

### Feat

- upgrade cli port to include blast arguments as parameters

## v6.0.0 (2023-06-30)

### BREAKING CHANGE

- main

### Feat

- include a print function to output the user arguments on cli execution
- include taxonomy in results and review the taxid and rank information delivered to the output object
- include options to control the word-size during blast executions

## v5.0.1 (2023-06-25)

### Refactor

- **filter_rank_by_identity**: remove unwanted print from filter_rank_by_identity use-case

## v5.0.0 (2023-06-25)

### BREAKING CHANGE

- main

### Fix

- fix bug on filter taxonomies by rank

## v4.0.1 (2023-06-25)

### Fix

- remove the min_consensus argument which is not used

### Refactor

- rename blastn execution entity and adapter to match the target of the execution which is the blastn

## v4.0.0 (2023-06-25)

## v3.0.1 (2023-06-25)

### Refactor

- remove unused dependencies from packages crates
- split the project into sub-crates to better manage dependencies

## v3.0.0 (2023-06-24)

### BREAKING CHANGE

- main

### Feat

- passthrough all blast builder arguments to the adapter executor

## v2.0.0 (2023-06-24)

### Feat

- implements the system check and upgrade logs to turn it better for human users
- implements the consensus check for multiple query results
- implements the identity based filtration of the taxon rank
- upgrade the cli port that execute the blast with consensus
- update consensus results to include a more informative results object
- split the parallel blast execution from the main use-case
- wip - start implemetation of the consensus check for the blast results
- create a method to parse taxonomy literals as a internal object
- wip - create the project base
- initial-commit

### Fix

- change type of bit-score from int64 to float64
- fix image paths on readme file
- fix the sort order of imports of the identity based filtration
- remove old rustfmt file

### Refactor

- review the access control modifiers of the application
