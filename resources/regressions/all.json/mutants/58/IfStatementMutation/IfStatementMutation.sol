// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract IfStatementMutation {
    function myBooleanNegation(bool a) public pure returns (bool) {
	/// IfStatementMutation(`a` |==> `true`) of: `if (a) {`
	if (true) {
	    return true;
	}
	else {
	    return false;
	}
    }
}
