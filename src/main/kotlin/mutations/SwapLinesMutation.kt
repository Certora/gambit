package mutations

import data.SolNode
import kotlin.random.Random


object SwapLinesMutation: Mutation() {
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "Block" && node.statements() != null &&
               node.statements()!!.size > 1
    }


    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not a mutation point for SwapLinesMutation" }
        val statements = node.statements()!!.toMutableList()
        statements.shuffle(rand)

        return node.replaceMultiple(source,
            listOf(statements[0] to statements[1].getText(source),
                   statements[1] to statements[0].getText(source)))
    }
}
