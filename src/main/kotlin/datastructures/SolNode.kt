package data

import kotlinx.serialization.json.*

class TypeDescriptions(val element: JsonElement) {
    fun typeString(): String? {
        return element.jsonObject["typeString"]?.jsonPrimitive?.content
    }
}

// a thin wrapper around a json element
class SolNode(val element: JsonElement) {
    fun getObject(): JsonObject? {
        return element as? JsonObject
    }

    private fun getNode(fieldName: String): SolNode? {
        return getObject()?.get(fieldName)?.let { SolNode(it) }
    }
    private fun getString(fieldName: String): String? {
        return getObject()?.get(fieldName)?.jsonPrimitive?.content
    }

    fun src(): String? { return getString("src") }
    fun name(): String? { return getString("name") }
    fun nodeType(): String? { return getString("nodeType") }

    fun expression(): SolNode? { return getNode("expression") }

    fun operator(): String? { return getString("operator") }
    // operator children
    fun leftExpression(): SolNode? { return getNode("leftExpression") }
    fun rightExpression(): SolNode? { return getNode("rightExpression") }

    // assignment
    fun leftHandSide(): SolNode? { return getNode("leftHandSide") }
    fun rightHandSide(): SolNode? { return getNode("rightHandSide") }

    // function call
    fun arguments(): List<SolNode>? {
        return getObject()?.get("arguments")?.jsonArray?.map(::SolNode)
    }

    // block statements
    fun statements(): List<SolNode>? {
        return getObject()?.get("statements")?.jsonArray?.map(::SolNode)
    }

    // if statement children
    fun condition(): SolNode? { return getNode("condition") }
    fun trueBody(): SolNode? { return getNode("trueBody") }
    fun falseBody(): SolNode? { return getNode("falseBody") }
    

    fun getTypeDescriptions(): TypeDescriptions? {
        return getObject()?.get("typeDescriptions")?.let { TypeDescriptions(it) }
    }

    fun getBounds(): Pair<Int, Int> {
        val parts = src()?.split(":") ?: throw UnsupportedOperationException("Src was not present when replacing")
        val start = parts[0].toInt()
        return Pair(start, start+parts[1].toInt())
    }

    // Get the text corresponding to this node
    fun getText(source: ByteArray): String {
        val (startByte, endByte) = getBounds()
        return String(source.copyOfRange(startByte, endByte), Charsets.UTF_8)
    }

    // Replace existing text in the source for this node with new text
    fun replaceInSource(source: ByteArray, new: String): String {
        val (startByte, endByte) = getBounds()
        return replacePart(source, new, startByte, endByte)
    }

    data class Replacement(val start: Int, val end: Int, val new: String)

    // Replace the text of several nodes at once
    fun replaceMultiple(source: ByteArray, replacements: List<Pair<SolNode, String>>): String {
        val sorted = replacements.map { (node, new) ->
            val (start, end) = node.getBounds()
            Replacement(start, end, new)
        }.sortedBy { it.start }

        var newSource = source
        var curOffset = 0
        for (replacement in sorted) {
            val actualStart = replacement.start + curOffset
            val actualEnd = replacement.end + curOffset

            val replaceBytes = replacement.new.toByteArray()
            newSource = newSource.copyOfRange(0, actualStart) + replaceBytes + newSource.copyOfRange(actualEnd, newSource.size)

            val newOffset = replaceBytes.size - (replacement.end - replacement.start)
            curOffset += newOffset
        }
        return String(newSource, Charsets.UTF_8)
    }

    fun replacePart(source: ByteArray, new: String, start: Int, end: Int): String {
        return String(source.copyOfRange(0, start) + new.toByteArray() + source.copyOfRange(end, source.lastIndex), Charsets.UTF_8)
    }

    fun commentOut(source: ByteArray): String {
        var (startByte, endByte) = getBounds()
        val restOfString = String(source.copyOfRange(endByte, source.size), Charsets.UTF_8)
        val match = "^\\s*;".toRegex().find(restOfString)
        if (match != null) {
            endByte += restOfString.substring(0, match.range.last + 1).toByteArray().size
        }
        return replacePart(
            source,
            "/*" + String(source.copyOfRange(startByte, endByte), Charsets.UTF_8) + "*/",
            startByte,
            endByte
        )
    }

    // subtrees are skipped when skip returns true for a node
    // subtrees are only visited if accept returns true for them
    fun <T> traverse(
        visitor: (SolNode) -> T?,
        skip: (SolNode) -> Boolean = { false },
        accept: (SolNode) -> Boolean = { true }
    ): List<T> {
        val result = mutableListOf<T>()
        traverseInternal(visitor, skip, accept, false, result)
        return result
    }

    private fun <T> traverseInternal(
        visitor: (SolNode) -> T?,
        skip: (SolNode) -> Boolean,
        accept: (SolNode) -> Boolean,
        accepted: Boolean,
        acc: MutableList<T>
    ) {
        var newAccepted = accepted
        if (accept(this)) {
            newAccepted = true
        }
        if (skip(this)) {
            return
        }
        if (newAccepted) {
            val res = visitor(this)
            if (res != null) {
                acc.add(res)
            }
        }

        if (element is JsonObject) {
            element.jsonObject.forEach { (_, value) ->
                val child = SolNode(value)
                child.traverseInternal(visitor, skip, accept, newAccepted, acc)
            }
        } else if (element is JsonArray) {
            element.jsonArray.forEach {
                val child = SolNode(it)
                child.traverseInternal(visitor, skip, accept, newAccepted, acc)
            }
        }
    }
}