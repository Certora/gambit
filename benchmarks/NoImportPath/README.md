# README

This file tests the case where a configuration file doesn't have an import path.
This should print a warning:

```
Warning: No `import_paths` specified in config
    Adding default import path /Users/benku/Gambit/benchmarks/NoImportPath.
    To fix, add
        "import_paths": ["/Users/benku/Gambit/benchmarks/NoImportPath"],
    to benchmarks/NoImportPath/gambit.json
```

and generate mutants.