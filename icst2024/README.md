# ICST2024 Data Repo

This directory has the scripts for reproducing the plots for our [ICST 2024 paper](https://conf.researchr.org/details/icst-2024/icst-2024-industry/2/Towards-Mutation-guided-Test-Suites-for-Smart-Contracts).
You can find out more about ERCx's test suite [here](https://ercx.runtimeverification.com/). The mutants for this paper were generated using commit `88e145b`. It will produce 64 mutants; as we mentioned in the paper, we manually inspected them and used 48 for the paper's evaluation. The ones that were removed are undetectable because they were changes in an internal function that only impacted other internal functions.

We provide the following results here:

1. The `data` from running the original ERCx test suite for ERC20s and the minimized one
on the top contracts from the [Awesome Buggy dataset](https://github.com/sec-bit/awesome-buggy-erc20-tokens/blob/master/bad_tokens.top.csv).

2. The `plots` we generated from that data corresponding to Figure 7 and 8 in the paper.

3. Scripts to reproduce the plots under `python`.

## Running the scripts

To generate plots, run `make` and everything should 'just work'

## Things didn't 'just work'?

Run the following:

```bash
python -m virtualenv .venv
source .venv/bin/activate
pip install -r requirements.txt

python3 python/plot_test_failure_rates.py
python3 python/plot_time_analysis.py
```

If this fails then we are doomed but please reach out to us and we will figure out the issue together ;)
