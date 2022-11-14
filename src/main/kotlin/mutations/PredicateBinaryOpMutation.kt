package mutations

import data.SolNode
import kotlin.random.Random


object PredicateBinaryOpMutation : BinaryOpMutation() {
    override val ops = setOf("==", "!=", ">", "<", ">=", "<=")
}
