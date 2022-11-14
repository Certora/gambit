# Certora Solidity Mutation Tester

This project is a mutation tester which checks that variants of the original
solidity program do not pass the specification. If a mutated program passes the specification, it may indicate that the specification is vacuous or not rigorous enough.


# Running the Mutation Tester

Example: `python3 scripts/mutationTest.py Test/Nirn Test/Nirn/run.sh --mutation-files Test/Nirn/CallInLoop.sol`

The tool expects 1) The folder of the test, which includes relavent solidity files, 2) The run script within that folder, and 3) Which files to mutation test

The entry point (`mutationTest.py`) is a script which simply calls the tool with these arguments.
The number of mutants to produce and types of mutants can be controlled:

Optional Arguments:
- `--num-mutants <number>`: number of mutants to produce per input solidity file
- `--mutations <type1> <type2> ...`: types of mutants to produce (Default all available kinds)
- `--seed <string>`: seed for the random number generator (Default 0)

Kinds of mutants:
- integer: Changes integer constants
- operator: Changes binary operators to constants or other operators
- deleteexpression: Deletes random expressions by commenting them out
- ifstatement: Changes the condition of if statements
- functioncall: Replaces function calls with argument to the function
- assignment: Changes the right hand side of assignments
- swaparguments: Swaps the arguments to function calls
- uncheckedblock: Adds an unchecked block around an expression
- require: Negates require statements
- swaplines: Swaps two lines in the code

# Picking Mutants

The mutation tester tool attempts to find `--num-mutants` mutants which compile using the solidity compiler.
It also distributes these mutants among the different enabled kinds of mutants.
For a given mutant, it finds mutants by randomly sampling uniformly from the set of 
all locations in a given program where a mutation can be applied.


# Adding New Mutants

Mutation types are located in `kotlin/mutations`. Each
implements two functions: `isMutationPoint` and `mutateRandomly`.
The first allows the sampler to find all the locations
where it makes sense to perform the mutation.
The second actually performs the mutation by returning a new
program.


# Implementation Details

The mutation tester works by the following process for each mutation file specified:

1) Get an AST for the original program (`MutationTestEntryPoint.kt`)

2) Make some mutants of the original program (`RunMutations.kt`):

    a) This samples uniformly from the mutation types which are enabled (all by default)

    b) For each mutation type, it samples uniformly from all the locations where the mutation can occur

3) Throw away mutants that do not compile (using the `validMutant` argument in `RunMutations.kt`)

5) In parallel, attempt to verify all of these mutants (`MutationTestEntryPoint.kt`)

6) Generate a report (`ReportRules.kt`)


In order to get mutants to compile and verify, the script generates temporary directories in the same place as the project folder. It copies all `.sol` and `.spec` files into the temporary directories, as well as the verification run script.
We also try to extract the version of solidity used from the verification script, but default to `solc`.

All of the mutants live in the `mutants` directory, and reporting is in the `report` directory.
