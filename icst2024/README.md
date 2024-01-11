# ICST2024 Data Repo

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

If this fails then we are doomed.