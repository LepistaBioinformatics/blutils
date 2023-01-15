# BLUTILS

The `Blutils` tool allow user to run and generate consensus identities of Blast
results. Currently the BlastN is available.

## Check dependencies

Optionally you could check OS dependencies before run `Blutils`. Naturally BLutils
depends on Ncbi-Blast+ tool to be installed on the host system to perform
parallel blast search. To check if the host OS has these package installed run
the `Blutils` checker for linux systems:

```bash
blu check linux
```

Note: Currently the system check is available only for linux systems and assumes
that dependencies could be evoked directly from terminal.

## Blast execution

Blast execution try to reaches the full available CPU saturation. At the default
multithread blast execution mode, the full saturation is not reached. To run
Blast through `Blutils` it is possible. All the steps taken during this process
can be seen in the image below.

![Parallel Blast](arc/drawio/parallel-blast.png)

## Consensus generation

Different from consensus generations from [QIIME
2](https://docs.qiime2.org/2022.11/), the `Blutils` consensus algorithm performs
a data pre-filtering based on Blast results for bit-score and perc-identity,
seems the algorithm described in the image below.

![Consensus Generation](arc/drawio/consensus-generation.png)
