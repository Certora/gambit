// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

// This contract provides test functions for relational operator replacement (ROR)
contract EVR {
    function add(uint256 a, uint256 b) public pure returns (uint256) {
        return a + b;
    }

    function evr_test_1(uint256 a, uint256 b) public pure returns (uint256) {
        /// ExpressionValueReplacement(`result` |==> `0`) of: `return result;`
        uint256 result = add(a, b);
        return 0;
    }

    function evr_test_2(uint256 a, uint256 b) public pure returns (uint256) {
        bool c = a < b;
        while (c) {
            b = b - a;
            c = a < b;
        }
        return a - b;
    }
}