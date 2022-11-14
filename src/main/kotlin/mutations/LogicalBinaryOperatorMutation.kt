package mutations

import data.SolNode
import kotlin.random.Random


object LogicalBinaryOperatorMutation : BinaryOpMutation() {
    override val ops = setOf("&&", "||", "&", "|", "^", "<<", ">>")
}
