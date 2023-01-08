# Blutils TODO list

## Project

- [ ] Write an appropriate README file.
    -  **location**: The project root.

## Adapters

- [ ] Create adapter to run blast into remote server. This feature is usual when
  High Performance Computing servers are available.
    - **location**: In `src/adapters` folder create an additional module for
    remote servers communication.

## Use cases

- [ ] Do implement the consensus generation when multiple `BlastResultRow`
  records returned after filtrate blast results by bit-score.
    - **location**: `src/use_cases/build_consensus_identities.rs` file at the
    `find_single_query_consensus` function.

## Ports (CLI)

- [ ] Create commands to extract databases (fasta and taxonomies) from taxdump
  and official NCBI databases.
    - **location**: In `src/ports/cli/cmds` create an additional module for
    parsing commands.
    - NOTE: Resolution of these TODO item should includes the creation of
      additional `use-cases` that execute desired actions.
