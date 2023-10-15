// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;

import "@lib/Lib.sol";
import "contracts/B.sol";

contract Contract {
    /// ArithmeticOperatorReplacement(`+` |==> `*`) of: `return a + b;`
    function plus(int256 a, int256 b) public pure returns (int256) {
        return a * b;
    }
}
