// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

// This contract provides test functions for relational operator replacement (ROR)
contract LVR {
    uint256 one_u = 1;
    uint256 zero_u = 0;
    int256 n_one_s = -1;
    int256 one_s = 1;
    int256 zero_s = 0;

    // Expect 1 mutant: 1
    function unsigned_zero() public pure returns (uint256) {
        uint256 zero = 0;
        return zero;
    }

    // Expect 2 mutant: 0, 2
    function unsigned_one() public pure returns (uint256) {
        uint256 one = 1;
        return one;
    }

    // Expect 2 mutants: 0, 1
    function signed_neg_one() public pure returns (int256) {
        int256 neg_one = -1;
        return neg_one;
    }

    // Expect 2 mutants: -1, 0
    function signed_pos_one() public pure returns (int256) {
        int256 pos_one = 1;
        return pos_one;
    }

    // Expect 2 mutants: -1, 1
    function signed_zero() public pure returns (int256) {
        /// ExpressionValueReplacement(`zero` |==> `0`) of: `return zero;`
        int256 zero = 0;
        return 0;
    }
}
