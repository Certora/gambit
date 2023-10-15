// SPDX-License-Identifier: GPL-3.0-only

pragma solidity ^0.8.13;

contract Fallback {
    function checkArgumentsAreReplaced(
        uint256 num,
        address addr,
        bool b
    ) public returns (uint256) {
        if (num == 0) {
            return 0;
        /// ExpressionValueReplacement(`checkArgumentsAreReplaced(num - 1, addr, !b)` |==> `0`) of: `return checkArgumentsAreReplaced(num - 1, addr, !b);`
        } else {
            return 0;
        }
    }
}
