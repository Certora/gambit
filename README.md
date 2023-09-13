<!-- SUPPRESS --> 

<!-- NOTE: IF YOU EDIT THIS FILE!!!

  # Keep In Sync With ReadTheDocs


  This documentation appears in two locations: 
  - here (the Gambit repo), and
  - https://github.com/Certora/Documentation/docs/gambit/gambit.md (used to
    generate the ReadTheDocs documentation at
    https://docs.certora.com/en/latest/docs/gambit/gambit.html)

  Unfortunately we cannot simply copy this file the Certora Documentation repo:
  the formats are slightly different (I describe how below). To solve this we
  _generate_ the RTD docs from this file.


  ## Steps to Sync With RTD
  

  1. Run `python scripts/generate_rtd_markdown.py README.md`

     This will create `./gambit.md`

  2. Move this generated `gambit.md` to location `docs/gambit/gambit.md` in the
     Certora Documentation repository (https://github.com/Certora/Documentation)
  
  3. Create a new PR in https://github.com/Certora/Documentation with the new
     Gambit docs
  
  4. Create a new PR in Gambit repo. Note that CI will check that the RTD
     documentation is up to date (see section "Checking RTDs are Up To Date"
     below). If this fails, CI will also fail, and you will be unable to merge
     into `master` until changes to the Gambit README are propagated to the RTD
     docs.
  
  5. Once the PR from (3) is merged and CI is passing in this repository, merge
     the PR from (4) into master.


  ## Checking RTDs are Up To Date

  To check that the RTD Gambit docs are in sync with Gambit's README, run

  ```
  python scripts/check_rtd_docs_upt_to_date.py
  ```

  This will translate the Gambit README to a string, pull the RTD docs from the
  Github Repo, and do a equality check on the two strings.
  
  You can optionally specify a `--branch` argument to choose another branch in
  the Certora Documentation repo (default is `'master'`)


  ## Markdown Format Differences


  1. Internal Links:

     ```markdown
     [Link Title](#link-title) becomes
     ```
     
     ```markdown
     {ref}`link-title`
     ```

     Note that `link-title` needs to target an _anchor_, which we describe in
     item 2.

  2. Anchors:
     
     To link to internal locations the RTD documentation expects an _anchor_:

     ```markdown
     (heading-title)=
     ## Heading Title
     ```

     Then the 

     ```markdown
     {ref}`heading-title`
     ```
     
    internal link will point to the `## Heading Title` section.

    Raw markdown doesn't support this syntax, so we use an HTML comment
    containing the contents "ANCHOR (content-title)=":

    ```markdown
    <!-- ANCHOR (heading-title)=   -- >
    ```

    Note that I intentionally added a space to the close comment tag because
    that would terminate this entire comment :)
  
  3. Notes:

     RTD uses admonition-style notes:

     ```{note}
     Some note goes here
     ```

     We don't have access to this here, so I've implemented a simple system,
     where all notes begin with a line containing:

     ```markdown
     _**Note:**
     ```

     and end with a line ending with `_`:

     ```markdown
     and this is the last line of my note._
     ```

     So, a full note would look like:

     ```markdown
     _**Note:**
     This is a note. The opening tag is on its own line, and the closing italic
     is at the end of the final line. This is to make parsing easy, and to keep
     diffs minimal!_
     ```
     
-->

<!-- END SUPPRESS -->

<!-- EMIT:
<\!--
  WARNING: AUTO_GENERATED DOCUMENTATION

  The following documentation is automatically generated from the Gambit
  README.md located at https://github.com/Certora/Gambit/README.md. Please view
  this document for instructions on producing this file.
--\>
-->
# Gambit: Mutant Generation for Solidity

Gambit is a state-of-the-art mutant generation system for Solidity. Gambit
injects faults into a Solidity program by applying predefined syntactic
transformations, called _mutation _operators_, to the program's source code. The
resulting faulty programs, called _mutants_, are used to evaluate a test suite
or a specification: each mutant represents a potential bug in the program, and
stronger test suites and specifications should detect more mutants as faulty.

## Requirements

1. Gambit is written in Rust. You'll need to [install Rust and
   Cargo](https://www.rust-lang.org/tools/install) to build Gambit.
2. Gambit uses the `solc` Solidity compiler to validate generated mutants. By
  default Gambit looks for `solc` on `$PATH`. Users can specify a particular
  `solc` executable with the `--solc` option, or disable validation entirely
  with `gambit mutate --skip_validate` (see `gambit mutate --help` for more
  details).

## Installation

You can download prebuilt Gambit binaries for Mac and Linux from our
[releases](https://github.com/Certora/gambit/releases) page.

To build Gambit from source, clone [the Gambit repository](https://github.com/Certora/gambit) and run

```
cargo build --release
```

from this repository's root. This will create the `target/release/gambit`
binary. To install globally, run

```
cargo install --path .
```

This will build Gambit and install it to a globally visible
location on your `PATH`.

## Usage

Gambit has two main commands: the [`mutate` command](#the-mutate-command), which
is responsible for generating mutants, and the
[`summary` command](#the-summary-command), which allows the user to get a
high-level summary of a `gambit mutate` execution.


<!-- ANCHOR: (the-mutate-command)= -->
## The `mutate` command

Gambit's `mutate` command expects user-provided _mutation parameters_ describing
which files to mutate, which mutation operators to apply, and several other
options. By default, these mutation parameters are specified by the user with
[command line arguments](#running-mutate-with-command-line-arguments). To handle
more complex use cases, and to allow for easy reproducibility, Gambit
can read mutation parameters from a 
[JSON configuration file](#running-mutate-with-a-configuration-file) with the
`--json` argument.

The `mutate` command does the following:

1. **Parse:** Gambit begins by parsing the specified Solidity files provided on
   command line or in the configuration file

2. **Function filters:** The `mutate` command provides two ways to filter which
  functions are mutated: the `--functions` filter and the `--contract` filter.
  When `--functions` is specified, Gambit will only mutate functions with a name
  contained in the provided list of functions. When `--contract` is specified,
  Gambit will only mutate functions within the specified contract. If neither
  option is specified, Gambit will mutate all functions.

3. **Mutation:** Gambit recursively visits the body of each function not
   filtered out in (2) and applies the mutation operators specified by the user;
   if no mutation operators were specified then Gambit uses a default set of
   mutation operators.

4. **Validation:** By default Gambit will _validate_ each generated mutant by
   compiling it with the `solc` compiler. If compilation fails Gambit will not
   export the mutant to disk or report it in `gambit_results.json` or
   `mutants.log`. Validation can be skipped with the `--skip_validate` option.
   To log invalidated mutants, use the `--log_invalid` option.
   
5. **Random down sampling:** If the user provides the `--num_mutants n`
   argument, Gambit will randomly down sample to `n` mutants.
  
6. **Write to disk:** After all mutants are generated, validated, and optionally
   down sampled, the `mutate` command exports the generated mutants and writes
   to the output directory (`gambit_out` by default), and writes a summary of
   each mutant to `gambit_out/gambit_results.json`.

<!-- ANCHOR: (running-mutate-with-command-line-arguments)= -->
### Running  `mutate` with command line arguments

By default the `mutate` command expects mutation parameters to be specified
on the command line:

```
gambit mutate FILENAME [ARGS...]
```


<!-- ANCHOR: (mutate-command-line-interface-options) -->
#### `mutate` command line interface options

Gambit's `mutate` command line interface supports the following options:

| Option               | Description                                                                                                                  |
| :------------------- | :--------------------------------------------------------------------------------------------------------------------------- |
| `--contract`         | specify a specific contract name to mutate; by default mutate all contracts                                                  |
| `--functions`        | specify one or more functions to mutate; by default mutate all functions                                                     |
| `--log_invalid`      | log any invalid mutants found during validation                                                                              |
| `--mutations`        | specify one or more mutation operators to use; only generates mutants that are created using the specified operators         |
| `--no_export`        | do not export mutant sources to output directory                                                                             |
| `--no_overwrite`     | do not overwrite an output directory; if the output directory exists, print an error and exit                                |
| `--num_mutants`      | randomly downsample to a given number of mutants.                                                                            |
| `--outdir`           | specify Gambit's output directory (defaults to `gambit_out`)                                                                 |
| `--random_seed`      | use a random seed. Note that this overrides any value specified by `--seed`                                                  |
| `--seed`             | specify a random seed. For reproducibility, Gambit defaults to using the seed `0`. To randomize the seed use `--random_seed` |
| `--skip_validate`    | only generate mutants without validating them by compilation                                                                 |
| `--solc`             | specify a `solc` binary to use during validation                                                                             |
| `--solc_allow_paths` | passes a value to `solc`'s `--allow-paths` argument                                                                          |



<!-- ANCHOR: (running-mutate-with-a-configuration-file)= -->
### Running `mutate` with a Configuration File

Gambit allows the user to specify mutation parameters in a JSON file, allowing
the user to store complex parameters, or even multiple parameters at once.
To run `mutate` with a configuration file, use:

```
gambit mutate --json CONFIGURATION_JSON
```


A set of mutation parameters are stored as a JSON object mapping option names to
values:

```json
{
  "filename": "contracts/ERC20.sol",
  "outdir": "gambit_out",
  "no_overwrite": true,
  "num_mutants": 5,
  "import_paths": ["imports1", "imports2"],
  "import_maps": ["a=x/a", "b=x/b"]
}
```


Gambit also supports specifying multiple sets of mutation parameters in a file.
Instead of a single JSON object, your configuration file should contain an
array of objects:

```json
[
    {
        "filename": "Foo.sol",
        "contract": "C",
        "functions": ["bar", "baz"],
        "solc": "solc8.12",
        "solc_optimize": true
    },
    {
        "filename": "Blip.sol",
        "contract": "D",
        "functions": ["bang"],
        "solc": "solc8.12"
    }
]
```

#### Paths in Configuration Files

Relative paths in a Gambit configuration file are _relative to the parent
directory of the configuration file_. This allows Gambit to be run from any
location without affecting the build configuration.

_**Warning:**
Remapping targets are **not relative paths**! If you specify a remapping
`@map=expanded/@map`, the target `expanded/@map` doesn't need to be a valid
path.  Instead, it needs to be be valid when extending a provided `import_path`.
So if the only import path is `.`, then `./expanded/@map` has to exist. But if
import paths `contracts` and `modules` are given, then one of either
`contracts/expanded/@map` or `modules/expanded/@map` needs to exist._

### Import Paths and Remappings

Gambit resolves imports while parsing, and this requires that you specify any
import paths and remappings that you would pass to `solc`.

Instead of `solc`'s `--base-path` and `--include-path` arguments, Gambit uses
a simpler scheme and replaces both of these with a single `--import_paths`
argument. For instance, if the `solc` invocation is `solc C.sol --base-path .
--include-path modules` , then the Gambit invocation becomes `gambit mutate C.sol
--import_paths . modules`.

Remappings are specified with the `--import_maps` argument. If the `solc`
invocation is `solc C.sol @openzeppelin=node_modules/@openzeppelin`, then the
Gambit invocation becomes `gambit mutate C.sol --import_maps
@openzeppelin=node_modules/@openzeppelin`.

### Mutant Validation

Gambit uses provided import paths and import remappings to invoke `solc`. For
instance, if you invoke `gambit mutate C.sol --import_paths A B C --import_maps
@x=y/@x`, then Gambit will validate a generated mutant by calling `solc
MutatedC.sol --base-path A/ --include-path B/ --include-path C/ @x=y/@x`.  If
you need to specify a `solc` `--allow-paths` argument, use the `mutate`
command's `--solc_allow_paths` argument.

<!-- ANCHOR: (the-summary-command)= -->
## The `summary` command

The `summary` command allows the user to see a summary of a `mutate` run:

```
gambit mutate benchmarks/Ops/AOR/AOR.sol
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
Generated 27 mutants in 0.41 seconds
</pre>

```
gambit summary
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
STD:      5 ( 18.52%)
AOR:     22 ( 81.48%)
---------------------
TOT:     27 (100.00%)
</pre>

To print the diffs of specific mutants, pass the `--mids` option:

```
$ gambit summary --mids 1 2
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
             === Mutant ID: 1 [StatementDeletion] ===

--- original
+++ mutant
@@ -9,8 +9,9 @@
     // a * b
     // a / b
     // a % b
+    /// StatementDeletion(`return a + b` |==> `assert(true)`) of: `return a + b;`
     function plus(int256 a, int256 b) public pure returns (int256) {
-        return a + b;
+        assert(true);
     }

     // Expect 4 mutants:

Path: mutants/1/benchmarks/Ops/AOR/AOR.sol


             === Mutant ID: 2 [ArithmeticOperatorReplacement] ===

--- original
+++ mutant
@@ -9,8 +9,9 @@
     // a * b
     // a / b
     // a % b
+    /// ArithmeticOperatorReplacement(`+` |==> `-`) of: `return a + b;`
     function plus(int256 a, int256 b) public pure returns (int256) {
-        return a + b;
+        return a - b;
     }

     // Expect 4 mutants:

Path: mutants/2/benchmarks/Ops/AOR/AOR.sol
</pre>

Pass the `--short` option to print a shorter summary of each mutant:

```
$ gambit summary --mids 1 2 3 4 5 --short
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
(1) STD [mutants/1/benchmarks/Ops/AOR/AOR.sol@13:9] return a + b -> assert(true)
(2) AOR [mutants/2/benchmarks/Ops/AOR/AOR.sol@13:18] + -> -
(3) AOR [mutants/3/benchmarks/Ops/AOR/AOR.sol@13:18] + -> *
(4) AOR [mutants/4/benchmarks/Ops/AOR/AOR.sol@13:18] + -> /
(5) AOR [mutants/5/benchmarks/Ops/AOR/AOR.sol@13:18] + -> %
</pre>

_**Note:**
The `summary` command is currently experimental, and its output and interface
may change in future releases._

<!-- ANCHOR: (results-directory)= -->
## Results Directory

`gambit mutate` produces all results in an output directory (default:
`gambit_out`). Here is an example:

```bash
gambit mutate benchmarks/Ops/AOR/AOR.sol -n 5
tree gambit_out -L 2
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
Generated 5 mutants in 0.15 seconds

gambit_out
├── gambit_results.json
├── input_json
├── mutants
│   ├── 1
│   ├── 2
│   ├── 3
│   ├── 4
│   └── 5
└── mutants.log

</pre>

This has the following structure:
+ `gambit_results.json`: a JSON file with detailed results
+ `input_json/`: intermediate files produced by `solc` that are used during mutation
+ `mutants/`: exported mutants. Each mutant is in its own directory named after
  its mutant ID (mid) 1, 2, 3, ...
+ `mutants.log`: a log file with all mutant information. This is similar to
  `results.json` but in a different format and with different information


<!-- SUPPRESS -->
## Testing

Gambit has _unit tests_ and _regression tests_. Run unit tests with `cargo
test`.

_**Note:**
All unit tests (`cargo test`) are currently run using `solc8.13`. Tests may fail
if `solc` points at a different version of the compiler._

Run regression tests with `scripts/run_regressions.sh`.  This script runs
`gambit mutate` on all configuration files in `benchmarks/config-jsons` and
compares the output against the expected output in `resources/regressions`.

_**Note:**
To update regression tests (e.g., in case of new test cases, new mutation
operators, altered mutation operators, etc), use the
`scripts/make_regressions.sh` script._

<!-- END SUPPRESS -->


## Mutation Operators
Gambit implements the following mutation operators

| Mutation Operator                   | Description                                                     | Example                                      |
| ----------------------------------- | --------------------------------------------------------------- | -------------------------------------------- |
| **arithmetic-operator-replacement** | Replace an arithmetic operator with another                     | `a + b` -> `a - b`                           |
| **bitwise-operator-replacement**    | Replace a bitwise operator with another                         | `a ^ b` -> `a & b`                           |
| **elim-delegate-call**              | Change a `delegatecall()` to a `call()`                         | `_c.delegatecall(...)` -> `_c.call(...)`     |
| **expression-value-repalcement**    | **(Experimental)** Replace expression with a value of same type | `a + b * 3` -> `0`                           |
| **literal-value-replacement**       | Replace a literal value with another                            | `1` -> `0`                                   |
| **logical-operator-replacement**    | Replace a logical expression                                    | `a && b` -> `false`                          |
| **relational-operator-replacement** | Replace a relational expression                                 | `a < b` -> `true`                            |
| **shift-operator-replacement**      | Replace a shift operator with another                           | `a << b` -> `a >> b`                         |
| **unary-operator-replacement**      | Replace a unary operator with another                           | `-b` -> `~b`                                 |
| **statement-deletion**              | Replace a statement with a no-op (`assert(true)`)               | `self.checkInvariants();` -> `assert(true);` |

For more details on each mutation type, refer to the [full documentation](https://docs.certora.com/en/latest/docs/gambit/gambit.html#mutation-types).

<!-- SUPPRESS -->
## Contact
If you have ideas for interesting mutations or other features,
we encourage you to make a PR or [email](mailto:chandra@certora.com) us.

## Credits
We thank
[Oliver Flatt](https://www.oflatt.com/) and
[Vishal Canumalla](https://homes.cs.washington.edu/~vishalc/)
for their excellent contributions to an earlier prototype of Gambit.

<!-- END SUPPRESS -->

[config-examples]: https://github.com/Certora/gambit/blob/master/benchmarks/config-jsons/
[test6]: https://github.com/Certora/gambit/blob/master/benchmarks/config-jsons/test6.json
