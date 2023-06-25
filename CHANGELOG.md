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
