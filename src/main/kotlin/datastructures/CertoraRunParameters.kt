package data

import java.nio.file.Path
import kotlinx.serialization.json.*
import org.apache.commons.lang3.SystemUtils
import kotlin.io.path.name

/**
 * CertoraRunParameters wraps a JsonElement object obtained by reading
 * a certoraRun.py CONF (.conf) file.  The class provides an interface
 * useful for invoking certoraRun in mutation testing.
 *
 * @constructor Initializes the object with the JsonElement.
 * @param element a JsonElement obtained by JSON parsing the .conf
 * file.
 */
class CertoraRunParameters(element: JsonElement) {

    private var jsonParameters = element

    /**
     * Method to check if the parameters contains the 'staging' key/value pair.
     * @return true if the parameters contains the 'staging' pair.
     */
    fun hasStaging(): Boolean = jsonParameters.jsonObject.containsKey("staging")

    /**
     * Method to check if the parameters contains the 'debug' key/value pair.
     * @return true if the parameters contains the 'debug' pair.
     */
    private fun hasDebug(): Boolean = jsonParameters.jsonObject.containsKey("debug")


    /**
     * Method to check if the parameters contains the 'build_only' key/value pair.
     * @return true if the parameters contains the 'build_only' pair.
     */
    private fun hasBuildOnly(): Boolean = jsonParameters.jsonObject.containsKey("build_only")

    /**
     * Method to write the parameters to a file.
     * @param path the Path object for the new file.
     */
    fun writeTo(path: Path) {
        val file = path.toFile()
        file.writeText(jsonParameters.toString())
    }

    /**
     * Method to add the 'staging' key/value pair to the parameters.
     * @param branch the staging branch to use.  Defaults to "master"
     *
     * The method will only add the key/value for 'staging' if not
     * already present.
     */
    fun setStaging(branch : String = "master"): CertoraRunParameters {
        if (!hasStaging()) {
            val jsonMap = mutableMapOf<String, JsonElement>()
            jsonMap.putAll(jsonParameters.jsonObject.toMap())
            jsonMap["staging"] = JsonPrimitive(branch)
            return CertoraRunParameters(JsonObject(jsonMap))
        }
        return this
    }

    /**
     * Method to add the 'debug' key/value pair to the parameters.
     * The method will only add the key/value for 'debug' if not
     * already present.
     */
    fun setDebug(): CertoraRunParameters {
        if (!hasDebug()) {
            val jsonMap = mutableMapOf<String, JsonElement>()
            jsonMap.putAll(jsonParameters.jsonObject.toMap())
            jsonMap["debug"] = JsonPrimitive("")
            return CertoraRunParameters(JsonObject(jsonMap))
        }
        return this
    }

    /**
     * Method to add the 'build_only' key/value pair to the parameters.
     * The method will only add the key/value for 'build_only' if not
     * already present.
     */
    fun setBuildOnly(): CertoraRunParameters {
        if (!hasBuildOnly()) {
            val jsonMap = mutableMapOf<String, JsonElement>()
            jsonMap.putAll(jsonParameters.jsonObject.toMap())
            jsonMap["build_only"] = JsonPrimitive(true)
            return CertoraRunParameters(JsonObject(jsonMap))
        }
        return this
    }

    /**
     * @property solidityCompiler the name of the solidity compiler configured in the
     * parameters.
     *
     * If the configuration does not name a compiler, defaults to solc.exe on Windows
     * and solc everywhere else.
     */
    val solidityCompiler: String
        get() {
            val compiler = jsonParameters.jsonObject["solc"]?.jsonPrimitive?.content ?: "solc"
            return compiler + if (SystemUtils.IS_OS_WINDOWS) ".exe" else ""
        }
}
