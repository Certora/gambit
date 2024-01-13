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

# Point Density
xy = np.vstack([df["% OTf"], df["% MTf"]])
z = gaussian_kde(xy)(xy)


plt.figure(figsize=(8, 8))
plt.scatter(df["% OTf"], df["% MTf"], s=5*(8000 * z), color="red")


plt.title("Original vs Minimized Tests")
plt.xlabel("Original Test Failure Rate")
plt.ylabel("Minimized Test Failure Rate")
# plt.show()

# Write plot to file
plt.savefig(osp.join(PLOT_DIR, "OTvsMT.pdf"))
