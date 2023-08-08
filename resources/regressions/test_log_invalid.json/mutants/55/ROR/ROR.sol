// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

// This contract provides test functions for relational operator replacement (ROR)
contract ROR {
    // Expect 3 mutants: x <= y, x != y, false
    function less(uint256 x, uint256 y) public pure returns (bool) {
        return x < y;
    }

    // Expect 3 mutants: x < y, x == y, true
    function less_equal(uint256 x, uint256 y) public pure returns (bool) {
        return x <= y;
    }

    // Expect 3 mutants: x >= y, x != y, false
    /// RelationalOperatorReplacement(`x > y` |==> `false`) of: `return x > y;`
    function more(uint256 x, uint256 y) public pure returns (bool) {
        return false;
    }

    // Expect 3 mutants: x > y, x == y, true
    function more_equal(uint256 x, uint256 y) public pure returns (bool) {
        return x >= y;
    }

    // Expect 3 mutants: x >= y, x <= y, false
    function equal_ord(uint256 x, uint256 y) public pure returns (bool) {
        return x == y;
    }

    // Expect 2 mutants: true, false
    function equal_not_ord(bool x, bool y) public pure returns (bool) {
        return x == y;
    }

    // Expect 3 mutants: x > y, x < y, true
    function not_equal_ord(uint256 x, uint256 y) public pure returns (bool) {
        return x != y;
    }

    // Expect 2 mutants: true, false
    function not_equal_not_ord(bool x, bool y) public pure returns (bool) {
        return x != y;
    }

    // Expect 3 mutants: (x + y) > z, (x + y) == z, true
    function more_equal_over_aor(
        uint256 x,
        uint256 y,
        uint256 z
    ) public pure returns (bool) {
        return (x + y) >= z;
    }

    // Expect 3 mutants: (x + y) > z, (x + y) < z, true
    function not_equal_over_aor(
        uint256 x,
        uint256 y,
        uint256 z
    ) public pure returns (bool) {
        return (x + y) != z;
    }
}
