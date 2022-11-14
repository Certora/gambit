package mutations

import data.SolNode
import kotlin.random.Random


object UncheckedBlockMutation: Mutation() {
    override fun isMutationPoint(node: SolNode): Boolean {
        return node.nodeType() == "ExpressionStatement"
    }


    override fun mutateRandomly(node: SolNode, source: ByteArray, rand: Random): String {
        check(isMutationPoint(node)) { "Node is not a mutation point for UncheckedBlockMutation" }
        val (start, end) = node.getBounds()
        // try to replace the old semicolon
        return node.replacePart(source, "unchecked{ ${node.getText(source)}; }", start, end+1)
    }
}
