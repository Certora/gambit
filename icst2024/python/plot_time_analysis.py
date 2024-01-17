import matplotlib.pyplot as plt
import csv
from os import path as osp

THIS_DIR = osp.dirname(__file__)
ROOT_DIR = osp.join(THIS_DIR, "..")
DATA_DIR = osp.join(ROOT_DIR, "data")
PLOT_DIR = osp.join(ROOT_DIR, "plots")



def extract_data_from_csv(file_path):
    addresses = []
    ot_failed_tests = []
    mt_failed_tests = []
    ot_durations = []
    mt_durations = []

    with open(file_path, 'r') as csv_file:
        csv_reader = csv.DictReader(csv_file)
        for row in csv_reader:
            addresses.append(row['symbol'])
            ot_failed_tests.append(float(row['% OTf']))
            mt_failed_tests.append(float(row['% MTf']))
            ot_durations.append(row['OT duration'])
            mt_durations.append(row['MT duration'])

    return addresses, ot_durations, mt_durations, ot_failed_tests, mt_failed_tests

def convert_to_seconds(duration):
    return float(duration[:-1])

def calculate_average_difference(ot_failed_tests, mt_failed_tests):
    differences = [ot - mt for ot, mt in zip(ot_failed_tests, mt_failed_tests)]
    average_difference = sum(differences) / len(differences)
    return average_difference

def truncate_address(address):
    return address[:12]

def create_comparison_graph(addresses, ot_durations, mt_durations, output_file="fig.png"):
    ot_seconds = [convert_to_seconds(ot) for ot in ot_durations]
    mt_seconds = [convert_to_seconds(mt) for mt in mt_durations]

    plt.figure(figsize=(30, 10))
    
    bar_width = 0.4
    plt.bar([truncate_address(addr) for addr in addresses], ot_seconds, width=bar_width, label='OT Duration', color='blue', alpha=0.7)
    plt.bar([truncate_address(addr) for addr in addresses], mt_seconds, width=bar_width, label='MT Duration', color='red', alpha=0.7)
    
    plt.xlabel('Token Name')
    plt.ylabel('Duration (seconds)')
    plt.title('OT and MT Durations Comparison')
    plt.legend()
    plt.xticks(rotation=45, ha='right')
    
    plt.savefig(output_file)
    plt.show()
    plt.close() 

def main():
    file_path = osp.join(DATA_DIR, "OTvsMT.csv")
    plot_path = osp.join(PLOT_DIR, "OTvsMTtimegraph.pdf")
    addresses, ot_durations, mt_durations, ot_failed_tests, mt_failed_tests = extract_data_from_csv(file_path)
    create_comparison_graph(addresses, ot_durations, mt_durations, output_file=plot_path)
    average_difference = calculate_average_difference(ot_failed_tests, mt_failed_tests)
    print(f"% OTf - % MTf : {round(average_difference, 2)}%")

main()
