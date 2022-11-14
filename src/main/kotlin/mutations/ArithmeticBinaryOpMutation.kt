package mutations

import data.SolNode
import kotlin.random.Random


object ArithmeticBinaryOpMutation : BinaryOpMutation() {
    override val ops = setOf("+", "-", "*", "/", "%", "**")
}
