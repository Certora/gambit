// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

// Unary Operator Replacement
contract UOR {
    // Expect no mutants: cannot negate an unsigned integer
    function unsigned_bw_not(uint256 x) public pure returns (uint256) {
        return ~x;
    }

    // Expect a single mutant: -x
    function signed_bw_not(int256 x) public pure returns (int256) {
        return ~x;
    }

    // Expect a single mutant: ~x
    /// UnaryOperatorReplacement(`` |==> ` ~ `) of: `return -x;`
    function signed_neg(int256 x) public pure returns (int256) {
        return  ~ -x;
    }
}
