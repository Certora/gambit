// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract BinaryOpMutation {
    function myAddition(uint256 x, uint256 y) public pure returns (uint256) {
	/// BinaryOpMutation(`+` |==> `%`) of: `return x + y;`
	return x%y;
    }

    function mySubtraction(uint256 x, uint256 y) public pure returns (uint256) {
	return x - y;
    }

    function myMultiplication(uint256 x, uint256 y) public pure returns (uint256) {
	return x * y;
    }

    function myDivision(uint256 x, uint256 y) public pure returns (uint256) {
	return x / y;
    }

    function myModulo(uint256 x, uint256 y) public pure returns (uint256) {
	return x % y;
    }

    function myExponentiation(uint256 x, uint256 y) public pure returns (uint256) {
	return x ** y;
    }

}
