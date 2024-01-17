.ONESHELL:
.PHONY: install

plots: clean install plots/OTvsMT_time_graph.png plots/OTvsMT.png

install: venv
	. .venv/bin/activate
	pip3 install -r requirements.txt

venv:
	python3 -m virtualenv .venv

plots/OTvsMT_time_graph.png:
	python3 python/plot_time_analysis.py

plots/OTvsMT.png:
	python3 python/plot_test_failure_rates.py



clean:
	rm -f plots/*