// SPDX-License-Identifier: GPL-3.0-only
pragma solidity ^0.8.13;

contract B {
    uint public num;
    address public sender;
    uint public value;

    function setVars(uint _num) public payable {
        num = _num;
        sender = msg.sender;
        value = msg.value;
    }
}

contract A {
    uint public num;
    address public sender;
    uint public value;
    bool public delegateSuccessful;
    bytes public myData;
    

    function setVars(address _contract, uint _num) public payable {
        /// ElimDelegateMutation of: (bool success, bytes memory data) = _contract.delegatecall(
        (bool success, bytes memory data) = _contract.call(
            abi.encodeWithSignature("setVars(uint256)", _num)
        );
	delegateSuccessful = success;
	myData = data;
    }
}
