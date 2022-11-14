package mutations

import data.SolNode
import kotlin.random.Random
import kotlin.random.nextULong


object AssignmentMutation: Mutation() {
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "Assignment"
    }


    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not mutation point for AssignmentMutation" }
        val newVal = listOf("true", "false", "0", "1", rand.nextULong().toString()).random(rand)
        return node.rightHandSide()?.replaceInSource(source, newVal)
            ?: throw IllegalStateException("Could not find right hand side of assignment")
    }
}
