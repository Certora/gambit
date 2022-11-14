package mutations

import data.SolNode
import kotlin.random.Random


object SwapArgumentsFunctionMutation: Mutation() {
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "FunctionCall" && node.arguments()!!.size > 1
    }


    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not a mutation point for SwapArgumentsMutation" }
        val children = node.arguments()!!.toMutableList()
        children.shuffle(rand)
        return node.replaceMultiple(source,
            listOf(Pair(children[0], children[1].getText(source)),
                   Pair(children[1], children[0].getText(source))))
    }
}
