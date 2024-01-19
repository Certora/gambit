from os import path as osp
import os
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from scipy.stats import gaussian_kde

# Get script directory as offset
THIS_DIR = osp.dirname(__file__)
ROOT_DIR = osp.join(THIS_DIR, "..")
DATA_DIR = osp.join(ROOT_DIR, "data")
PLOT_DIR = osp.join(ROOT_DIR, "plots")
if not osp.exists(PLOT_DIR):
    os.makedirs(PLOT_DIR)
ORIG_VS_MINIMIZED_TESTS_CSV = osp.join(DATA_DIR, "OTvsMT.csv")

df = pd.read_csv(ORIG_VS_MINIMIZED_TESTS_CSV)

df["% OTf"] = pd.to_numeric(df["% OTf"], errors="coerce")
df["% MTf"] = pd.to_numeric(df["% MTf"], errors="coerce")
df["Failing tests OT"] = pd.to_numeric(df["Failing tests OT"], errors="coerce")
df["Failing tests MT"] = pd.to_numeric(df["Failing tests MT"], errors="coerce")

groupings = {}
# Iterate through rows and group by (Failing tests OT, Failing tests MT)
for index, row in df.iterrows():
    key = (row["Failing tests OT"], row["Failing tests MT"])
    if key not in groupings:
        groupings[key] = []
    groupings[key].append(row)

sizes = []
otfs = []
mtfs = []

for key, rows in groupings.items():
    row = rows[0]
    otf = row["% OTf"]
    mtf = row["% MTf"]

    otfs.append(otf)
    mtfs.append(mtf)
    sizes.append(len(rows))

print("Sizes:")
print(sizes, sum(sizes))

# Scale sizes so the areas increase linear to the bubble size
sizes = [s**0.5 for s in sizes]

print("Scaled sizes:")
print(sizes)
sizes = np.array(sizes)

plt.figure(figsize=(8, 8))
plt.scatter(otfs, mtfs, alpha=0.5, s=30 * sizes, color="red")

plt.title("Failure Rates of Full vs Minimized Tests")
plt.xlabel("Full Test Suite Failure Rate")
plt.ylabel("Minimized Test Suite Failure Rate")
# plt.show()

# Write plot to file
plt.savefig(osp.join(PLOT_DIR, "OTvsMT.pdf"))
