import sys
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Rectangle

# Lire les données du fichier CSV
file1 = sys.argv[1]
data1 = pd.read_csv(file1, index_col=0)

# Calculer la matrice de corrélation
corr_matrix = data1.corr()

# Créer la figure et l'axe pour la heatmap
fig, ax = plt.subplots(figsize=(10, 10))
im = ax.imshow(corr_matrix, cmap='coolwarm', vmin=-1, vmax=1)

ax.set_xticks(np.arange(len(data1.columns)))
ax.set_yticks(np.arange(len(data1.columns)))

ax.set_xticks(np.arange(len(data1.columns) + 1) - 0.5, minor=True)
ax.set_yticks(np.arange(len(data1.columns) + 1) - 0.5, minor=True)
ax.grid(which='minor', color='black', linestyle='-', linewidth=1)

cbar = ax.figure.colorbar(im, ax=ax)

block_size = 1.0
correlated_tests = set()

for i in range(len(data1.columns)):
    for j in range(len(data1.columns)):
        corr_value = corr_matrix.iloc[i, j]
        if -0.25 <= corr_value <= 0.25:
            rect = Rectangle((j - block_size/2, i - block_size/2), block_size, block_size, fill=True, facecolor='white')
            ax.add_patch(rect)
        if corr_value > 0.90:
            if data1.columns[i] != data1.columns[j]:
                correlated_tests.add(frozenset([data1.columns[i], data1.columns[j]]))

with open('correlated_tests.txt', 'w') as f:
    for pair in correlated_tests:
        test1, test2 = pair
        f.write(test1 + ' & ' + test2 + '\n')

plt.tight_layout()

plt.savefig('figure.svg', format='svg', dpi=300, bbox_inches='tight')

plt.show()
