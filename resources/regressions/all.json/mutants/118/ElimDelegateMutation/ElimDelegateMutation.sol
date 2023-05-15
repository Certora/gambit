// SPDX-License-Identifier: GPL-3.0-only
pragma solidity ^0.8.13;

contract B {
    uint public num;
    address public sender;
    uint public value;

    function setVars(uint _num) public payable {
        num = _num;
        sender = msg.sender;
        /// AssignmentMutation(`msg.value` |==> `0`) of: `value = msg.value;`
        value = 0;
    }
}

contract A {
    uint public num;
    address public sender;
    uint public value;
    bool public delegateSuccessful;
    bytes public myData;
    

    function setVars(address _contract, uint _num) public payable {
        (bool success, bytes memory data) = _contract.delegatecall(
            abi.encodeWithSignature("setVars(uint256)", _num)
        );
	delegateSuccessful = success;
	myData = data;
    }
}
