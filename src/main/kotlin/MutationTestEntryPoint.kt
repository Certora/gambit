import com.github.ajalt.clikt.core.CliktCommand
import com.github.ajalt.clikt.core.FileNotFound
import com.github.ajalt.clikt.parameters.arguments.argument
import com.github.ajalt.clikt.parameters.options.*
import com.github.ajalt.clikt.parameters.types.file
import com.github.ajalt.clikt.parameters.types.int
import com.github.kittinunf.fuel.Fuel
import com.github.kittinunf.fuel.core.Headers
import com.github.kittinunf.fuel.core.isSuccessful
import data.CertoraRunParameters
import data.SolNode
import kotlinx.serialization.json.*
import log.Logger
import log.LoggerTypes
import mutations.RunMutations
import mutations.allMutations
import mutations.mutationTypesFromStrings
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream
import parallel.ParallelPool
import parallel.Scheduler.compute
import parallel.forkEvery
import parallel.pcompute
import java.io.*
import java.lang.IllegalStateException
import java.lang.Integer.max
import java.net.URL
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import kotlin.io.path.*
import kotlin.random.Random
import kotlin.streams.toList

class MutationTest : CliktCommand() {
    private val projectFolder by argument(help = "The folder containing the project to test").file(mustExist = true)
    private val runScript by argument(help = "The script which runs the verification process").file(mustExist = true)
    private val mutationFiles by option("--mutation-files",
        help = "The files which should be mutated.").multiple(required = true)
    private val numMutants by option(
        "--num-mutants", help = "number of mutants to generate per solidity file").int().default(20)
    private val mutations by option(
        "--mutations", help = "Mutations to use by their class name. Mutants can be specified with or without 'mutation' at the end of the name.").multiple()
    private val functions by option(
        "--functions", help = "Functions to mutate by their name.").multiple()
    private val numThreads by option(
        "--num-threads", help = "number of threads to use").int().default(max(1, Runtime.getRuntime().availableProcessors()/2))
    private val seed by option(
        "--seed", help = "seed to use for random number generator").default("0")
    private val mutantCompileFolder by option(
        "--mutant-compile-folder", help = "temporary folder in which to compile mutants").default("Compile")
    private val printFailedCompile by option(
        "--print-failed-compile", help = "Print the diff for mutants that fail to compile").flag()
    private val manualMutations by option(
        "--manual-mutations", help = "Specify a folder of mutations to use, one folder per mutation file.").multiple()

    /**
     * CloudInfo contains the branch name for staging and
     * a Boolean that indicates whether to use the staging cloud
     * or not.
     */
    private data class CloudInfo(
        val useCloud : Boolean = false,
        val branch : String = "master"
    )

    private val staging by option(
        "--staging", help="Verify mutants with verifier in the cloud").convert { CloudInfo(true, it) }
        .default(CloudInfo())

    private val useCLICertoraRun by option(
        "--use-cli-certorarun", help="Use CLI certoraRun rather than certoraRun.py").flag(default = false)

    /**
     * The command to use for certoraRun.py based on the command line flag useCLICertoraRun.
     * Initialized in getCertoraRunparameters() because the useCLICertoraRun option delegate
     * must read the command-line prior to reading its value.
     */
    private val certoraRunCommand : String by lazy {
        if (useCLICertoraRun) "certoraRun" else "certoraRun.py"
    }

    private val certoraRunParameters: CertoraRunParameters by lazy {
        getConf()
    }

    /**
     * Logger for info/warn level messages.  The class also uses Logger.always in places.
     */
    private val logger = Logger(LoggerTypes.MUTATION_TESTER)

    /**
     * Method to return the command for running the Solidity compiler executable
     * by loading it from the certoraRun parameters.
     *
     * Defaults to "solc" or "solc.exe" if running on Windows.
     */
    private fun solc(): String {
        return certoraRunParameters.solidityCompiler
    }

    // Run a command and capture the output, error, and exit code
    private fun runCommand(
        args: List<String>,
        stdin: String,
        workingDir: Path = Paths.get("."),
        script: File? = null
    ): Triple<String, String, Int> {
        Logger.always(
            "Running $args from $workingDir and ${if (script != null) script.name else "no"} script",
            respectQuiet = true
        )
        val process = ProcessBuilder(*args.toTypedArray())
            .redirectOutput(ProcessBuilder.Redirect.PIPE)
            .redirectInput(ProcessBuilder.Redirect.PIPE)
            // TODO why does Redirect.INHERIT not work? For now we just capture it
            .redirectError(ProcessBuilder.Redirect.PIPE)
            .directory(workingDir.toFile())
            .start()
        val procOutput = process.outputStream.bufferedWriter()
        val procInput = process.inputStream.bufferedReader()
        val procError = process.errorStream.bufferedReader()
        procOutput.write(stdin)
        if (script != null) {
            script.reader().transferTo(procOutput)
            procOutput.write("\n")
        }
        procOutput.flush()
        procOutput.close()
        val res = procInput.readText()
        procInput.close()
        val error = procError.readText()
        procError.close()
        val exitcode = process.waitFor()
        return Triple(res, error, exitcode)
    }

    private fun doesProgramCompile(compileFile: Path): Boolean {
        return runCommand(listOf(solc(), compileFile.absolutePathString()), "").third == 0
    }

    private fun copySolRecursively(from: Path, to: Path) {
        // only copy these types of files
        val extensions = setOf("sol", "spec")

        from.toFile().walkTopDown().onEnter {
            !it.name.startsWith("emv")
        }.forEach {
            val rel = it.toPath().relativeTo(from)
            val dest = to.resolve(rel)
            if (dest.extension in extensions || dest.fileName.toString() == runScript.name) {
                Files.createDirectories(dest.parent)
                Files.copy(it.toPath(), dest)
            }
        }
    }

    // Copy over to a temporary directory for a mutant, deleting any existing folder found
    private fun copyFolderForMutant(mutant: String): Path {
        val mutantFolder = projectFolder().parent.resolve("Mutant$mutant")
        check(mutantFolder.toFile().deleteRecursively()) { "failed to delete temporary folder ${mutantFolder.absolutePathString()}" }
        copySolRecursively(projectFolder(), mutantFolder)
        return mutantFolder
    }

    /**
     * @param folder in which to search for the output.json
     * @return path of the latest output.json file in the emv-../Reports directory.
     */
    private fun getLatestOutputJson(folder: Path): Path? {
        // TODO: in the next revision, we should use the treeview.json.
        return Files.newDirectoryStream(folder, "emv-*").use {
            it.maxByOrNull { it.toFile().lastModified() }?.resolve("Reports/output.json")
        }
    }

    /**
     * @param projDir path of the project directory.
     * @return mapping from files names with .json.stdout extensions
     * to the content parsed as a json object.
     * */
    private fun getSourceJsons(): Map<String, SolNode> {
        Logger.always("Generating AST json files...", respectQuiet = true)
        val paramsBuildOnly = certoraRunParameters.setBuildOnly().setDebug()
        val certoraConfigFile = projectFolder().resolve("build_only.conf")
        paramsBuildOnly.writeTo(certoraConfigFile)
        runCommand(listOf(certoraRunCommand, certoraConfigFile.fileName.toString()), "", projectFolder())
        val configDir = Files.newDirectoryStream(projectFolder().resolve(".certora_internal")).use {
            it.maxByOrNull { it.toFile().lastModified() }?.resolve(".certora_config")
        }
        if (configDir != null) {
            val astJsons = Files.walk(configDir)
                .filter {
                    val nm = it.fileName.toString()
                    // TODO: please don't do this. Must be a way to figure out if a file autogenerated.
                    nm.endsWith(".json.stdout") && !nm.startsWith("autoFinder_")
                }
                .toList()
            val fileToJsonAST: Map<String, SolNode> =
                astJsons.associate { it.name to SolNode(Json.parseToJsonElement(Files.readString(it))) }
            astJsons.map { it.deleteIfExists() }
            Files.delete(certoraConfigFile)
            Logger.always("Dumped AST jsons ...", respectQuiet = true)
            return fileToJsonAST
        } else {
            throw IllegalStateException("No Files were generated in .certora_internal")
        }
    }

    /**
     * Method to download the results (the tarball) of a cloud verifier run from the named
     * url and save the results in the named file.
     * @param url the url to use to retrieve the tarball.
     * @param path the path to the file to store the tarball.
     */
    private fun downloadVerificationReport(url: URL, path: Path) {
        val certoraKey = System.getenv("CERTORAKEY")
        var tries = 10
        var succeeded = false
        while (tries-- > 0) {
            val request = Fuel.get(url.toString())
                .header(Headers.COOKIE to "certoraKey=$certoraKey")

            // resp is a Triple of type Triple<Request, Response, Result<ByteArray, FuelError>>
            val resp = request.response()

            if (resp.second.isSuccessful) {
                // component1 is the ByteArray portion of the Result<ByteArray, FuelError> from the resp Triple.
                // Result<A,B> is a class used by the Fuel library, not sure why it doesn't just use a Pair. In
                // any case, the ByteArray portion holds the actual bytes of the tarball.
                val byteArray = resp.third.component1()
                if (byteArray != null) {
                    path.toFile().writeBytes(byteArray)
                    succeeded = true
                    break
                }
            }
            else {
                Logger.always("Failed to retrieve verification report, remaining tries: $tries", respectQuiet = true)
            }
        }
        check(succeeded) { "Did not retrieve verification report from $url}" }
    }

    /**
     * Helper function to convert a FileInputStream to a GzipCompressionInputStream
     */
    private fun FileInputStream.gzipCompressorStream(): GzipCompressorInputStream = GzipCompressorInputStream(this)

    /**
     * Helper function to convert an GzipCompressorInputStream to a TarArchiveInputStream
     */
    private fun GzipCompressorInputStream.tarArchiveStream(): TarArchiveInputStream = TarArchiveInputStream(this)

    /**
     * Helper function to convert a FileOutputStream to a BufferedOutputStream
     */
    private fun FileOutputStream.bufferedOutputStream(): BufferedOutputStream = BufferedOutputStream(this)

    /**
     * Method to extract the contents of a cloud results tarball and place them in
     * a destination folder.
     * @param tarball the path to the results tarball.
     * @param destinationDirectory the path that should contain the results extracted
     * from the tarball.
     */
    private fun extractVerificationReportFromTarball(tarball: Path, destinationDirectory: Path) {
        Logger.always("Extracting verification result in $destinationDirectory", respectQuiet = true)
        val tarballParentDirectory = tarball.parent
        val extractedDirectory = tarballParentDirectory.resolve("TarName")
        FileInputStream(tarball.toString())
            .gzipCompressorStream()
            .tarArchiveStream()
            .use { tarInputStream ->
                var finished = false
                while (!finished) {
                    val entry = tarInputStream.nextTarEntry
                    if (entry == null) {
                        finished = true
                        continue
                    } else if (entry.isDirectory) {
                        if (entry.name == "debugs") {
                            continue
                        }
                        val dir = File(tarballParentDirectory.toString() + File.separator + entry.name)
                        if (!dir.mkdir()) {
                            logger.warn { "Unable to create directory ${entry.name} during report extraction" }
                        }
                    } else {
                        val outStream = FileOutputStream(
                            tarballParentDirectory.toString() + File.separator + entry.name,
                            false
                        ).bufferedOutputStream()
                        tarInputStream.transferTo(outStream)
                        outStream.flush()
                    }
                }
            }

        destinationDirectory.toFile().deleteRecursively()
        Files.move(extractedDirectory, destinationDirectory)
    }

    /**
     * Method to get the collect the output from running a mutant test in the cloud.
     * @param url the location of the file containing the tarball download URL
     * @param folder the path to the folder that should contain the extracted tarball results.
     *
     * The method does the following:
     * - reads the tarball url from the certoraRun.py generated file .zip-output-url.txt
     * - downloads the tarball.
     * - extracts the tarball contents to the folder location.
     *
     * certoraRun.py must generate a file called .zip-output-url.txt.
     *
     * It would be better to have a rest API to call to get the tarball.
     */
    private fun retrieveVerifierResultsFromCloud(url: Path, folder: Path, dest: Path) {
        // We need to check the output for the link to the tarball.
        val urlValue = url.toFile().readText()
        Logger.always("Retrieving verification report from $urlValue", respectQuiet = true)

        val fileName = folder.resolve("staging-results.tar.gz")
        val destinationDirectory = folder.resolve(dest)

        downloadVerificationReport(URL(urlValue), fileName)
        extractVerificationReportFromTarball(fileName, destinationDirectory)
        Logger.always("Successfully retrieved results from cloud", respectQuiet = true)
    }

    /**
     * Simple enum describing whether the verifier succesfully ran or crashed.
     */
    private enum class RunStatus {
        Succeeded,
        Crashed,
    }

    /**
     * Wrapper class that contains the results of running verification
     */
    private data class VerifierStatus(
        val status : RunStatus = RunStatus.Crashed,
        val standardOut : String = String(),
        val standardErr : String = String(),
        val exitCode : Int = -1
    )

    /**
     * Function to run the verification command.
     * @param command the list of command args to run
     * @param folder the temporary working folder
     * @param mutant the name of the mutant (used for logging purposes)
     * @param tries the number of times to re-try the verification
     * @param predicate the lambda used to evaluate whether the function should continue looping and re-trying.
     * @return a VerificationResult object.
     *
     * The function will run the verification [tries] times in order to get a result.  Each time, the function
     * invokes the predicate to determine if it should continue.  If the predicate returns true, the function
     * terminates.  If the predicate returns false, the function will try again.
     */
    private fun runVerification(
        command: List<String>,
        folder: Path,
        succeeded: (runResult: Triple<String, String, Int>) -> Boolean,
        tries: Int = 5
    ): VerifierStatus {
        var attempts = tries
        var verificationResult = VerifierStatus()
        while (attempts-- > 0) {
            val runNumber = tries - attempts
            val runNumberText = if (certoraRunParameters.hasStaging()) " (try #$runNumber/$tries)" else ""

            Logger.always("Running verification in folder $folder$runNumberText:", true)

            val runResult = runCommand(command, "", folder)

            if (succeeded(runResult)) {
                verificationResult = VerifierStatus(
                    RunStatus.Succeeded,
                    runResult.first,
                    runResult.second,
                    runResult.third
                )
                break
            } else {
                logger.warn {
                    "Verification failed for $folder$runNumberText: ${runResult.first}"
                }
            }
        }
        return verificationResult
    }

    /**
     * Verifies a mutant and sorts it into a folder based on if it passed or not
     * @param originalFile file to mutate.
     * @param path directory where the verification of the mutant for originalFile is to be done.
     * @return the output.json that has the results.
     */
    private fun verifyMutant(originalFile: Path, mutant: Path): JsonElement {
        Logger.always("Verifying $mutant for $originalFile", respectQuiet = true)
        val tempFolder = copyFolderForMutant(mutant.fileName.toString())
        val targetRelative = originalFile.relativeTo(projectFolder())
        val targetFile = tempFolder.resolve(targetRelative)
        Files.delete(targetFile)
        Files.copy(mutant, targetFile)

        // Write the updated certoraRun.py configuration to a temporary .conf file.
        val certoraConfigFile = tempFolder.resolve("verify-mutant.conf")
        certoraRunParameters.writeTo(certoraConfigFile)

        val element: JsonElement
        val resultsFile: Path?
        val mutantRes: Pair<VerifierStatus, Path?>
        if (certoraRunParameters.hasStaging()) {
            mutantRes = verifyOnStaging(certoraConfigFile, tempFolder)
            resultsFile = mutantRes.second?.resolve("Reports/output.json")
        } else {
            mutantRes = verifyAndGetResults(certoraConfigFile, tempFolder, tempFolder)
            resultsFile = getLatestOutputJson(tempFolder)
        }

        val (verifierStatus, _) = mutantRes
        val resFolder = if (verifierStatus.exitCode == 0) {
            Logger.always("Mutant PASSES verification: ${mutant.fileName}", respectQuiet = true)
            passedVerificationFolder()
        } else {
            Logger.always("Mutant fails verification: ${mutant.fileName}", respectQuiet = true)
            failedVerificationFolder()
        }

        Files.copy(mutant, resFolder.resolve(mutant.fileName))

        resFolder.resolve("${mutant.fileName}.output.txt").writeText(verifierStatus.standardOut)
        resFolder.resolve("${mutant.fileName}.error.txt").writeText(verifierStatus.standardErr)

        element = if (resultsFile == null) {
            JsonPrimitive("CRASHED")
        } else {
            Json.parseToJsonElement(Files.readString(resultsFile))
        }

        // clean up the temporary folder
        check(
            tempFolder.toFile().deleteRecursively()
        ) { "failed to delete temporary folder ${tempFolder.absolutePathString()}" }
        return element
    }

    /**
     * run the diff program on two files
     * @param mutant: mutated solidity source file.
     * @param original: original solidity source file.
     */
    private fun diffMutant(mutant: Path, original: Path) {
        Logger.always("Mutant ${mutant.absolutePathString()}:", respectQuiet = true)
        val diffcommand = listOf("diff", original.absolutePathString(), mutant.absolutePathString())
        val (difftext, _, exitcode) = runCommand(diffcommand, "")
        // check that there was a difference
        when (exitcode) {
            0 -> {
                Logger.always("Generated an identical mutant", respectQuiet = true)
            }
            1 -> {
                Logger.always(difftext, respectQuiet = true)
            }
            else -> {
                Logger.always("Install a 'diff' program to see the diff output", respectQuiet = true)
            }
        }
    }

    /**
     * The main entry point for each file in the project that should be mutated.
     * It generates mutants, makes sure they compile and returns a
     * list of paths that correspond to each valid mutant.
     * @param contractFile: file to mutate.
     * @param allAsts: ASTs of all the solidity files included in the verification task.
     * @param manualMutationFolder: directories for manual mutations if any.
     * @param rand to seed the random nodes chosen for mutation.
     * @return a list of directories for each mutated version of contractFile.
     */
    private fun mutateFile(
        contractFile: Path,
        allAsts: Map<String, SolNode>,
        manualMutationFolder: Path?,
        rand: Random
    ): List<Path> {
        Logger.always("Attempting to mutate file $contractFile", respectQuiet = true)
        // TODO: is it possible that the same AST appears in the "sources" field of multiple json files?
        val solNode = allAsts
            .map { (_, solNode) -> solNode.getObject() }
            .map { it?.get("sources")?.jsonObject }
            .firstOrNull { it?.keys?.contains(contractFile.toString()) ?: false }
            ?.get(contractFile.toString())
            ?.let { SolNode(it) }

        // by default, enable all mutations
        val mutationTypes = if (mutations.isEmpty()) {
            allMutations
        } else {
            mutationTypesFromStrings(mutations)
        }

        val targetRelative = contractFile.relativeTo(projectFolder())
        val compileFolder = copyFolderForMutant(mutantCompileFolder)
        val compileFile = compileFolder.resolve(targetRelative)
        if (solNode == null) {
            throw IllegalStateException("SolNode for $contractFile is null.")
        }
        val runObject = RunMutations(contractFile, solNode, rand, numMutants, allFolder(), functions)
        // Get a bunch of mutants, while checking they compile
        val randomMutations = runObject.getMutations(mutationTypes) { mutationFile: Path ->
            Logger.always(
                "Checking if mutant compiles: ${mutationFile.absolutePathString()}",
                respectQuiet = true
            )
            Files.delete(compileFile)
            Files.copy(mutationFile, compileFile)

            if (doesProgramCompile(compileFile)) {
                Logger.always("Mutant compiles: ${mutationFile.absolutePathString()}", respectQuiet = true)
                diffMutant(mutationFile, contractFile)
                true
            } else {
                Logger.always(
                    "Mutant failed to compile: ${mutationFile.absolutePathString()}",
                    respectQuiet = true
                )
                if (printFailedCompile) {
                    diffMutant(mutationFile, contractFile)
                }
                mutationFile.copyTo(compileErrorFolder().resolve(mutationFile.name))
                false
            }
        }

        // Ensure that the manual mutations compile
        val manualMutations = manualMutationFolder?.listDirectoryEntries()?.map {
            if (!doesProgramCompile(it)) {
                throw IllegalArgumentException("Manual mutation ${it.absolutePathString()} failed to compile")
            }

            Logger.always("Manual mutant ${it.absolutePathString()} compiles", respectQuiet = true)
            diffMutant(it, contractFile)
            it
        } ?: emptyList()

        // clean up the temporary compilation folder
        check(compileFolder.toFile().deleteRecursively()) { "failed to delete temporary folder ${compileFolder.absolutePathString()}" }
        return randomMutations.plus(manualMutations)
    }

    private fun runScript(): Path {
        return Paths.get(runScript.absolutePath)
    }
    private fun projectFolder(): Path {
        return Paths.get(projectFolder.absolutePath)
    }
    private fun outputFolder(): Path {
        return projectFolder().parent.resolve( projectFolder.name + "Mutants")
    }
    private fun allFolder(): Path {
        return outputFolder().resolve("all")
    }

    private fun compileErrorFolder(): Path {
        return outputFolder().resolve("compileError")
    }

    private fun passedVerificationFolder(): Path {
        return outputFolder().resolve("passedVerification")
    }

    private fun failedVerificationFolder(): Path {
        return outputFolder().resolve("failedVerification")
    }

    private fun resultsJsonFile(): Path {
        return outputFolder().resolve("results.json")
    }

    /** Verify a program and return the result and the path that contains the results.
     * @return a pair of the result and path where the result is stored.
     * @param configFile is the .conf file to use for verification.
     * @param folder from where the configFile is to be executed.
     * @param resultDest the directory where the result is generated.
     * @param isStaging flag to indidate if staging is enabled.
     */
    private fun verifyAndGetResults(configFile: Path, folder: Path, resultDest: Path, isStaging: Boolean = false): Pair<VerifierStatus, Path?>  {
        val command = listOf(certoraRunCommand, configFile.fileName.toString())
        val verificationResult = runVerification(command, folder, {
            if (isStaging) {
                val zipUrlFile = folder.resolve(".zip-output-url.txt")
                if (zipUrlFile.exists()) {
                    Logger.always("Zip url: $zipUrlFile exists", respectQuiet = true)
                    retrieveVerifierResultsFromCloud(zipUrlFile, folder, resultDest)
                    true
                }
                else {
                    Logger.always("Zip url: $zipUrlFile DOES NOT exists", respectQuiet = true)
                    false
                }
            }
            else {
                Logger.always("Running verification locally", respectQuiet = true)
                true
            }
        })
        return verificationResult to resultDest
    }

    /**
     * A wrapper around `verifyAndGetResults` to run verification on the staging platform.
     * @return a pair of the result and path where the result is stored.
     * @param certoraConfigFile is the .conf file to use for verification.
     * @param dir is the directory from where the configFile is to be executed.
     */
    private fun verifyOnStaging(certoraConfigFile: Path, dir: Path): Pair<VerifierStatus, Path?> {
        val stagedDest = dir.resolve("emv-staging-results")
        val verifierStatus = verifyAndGetResults(certoraConfigFile, projectFolder(), stagedDest, true)
        check(verifierStatus.first.status == RunStatus.Succeeded) {
            "Failed to retrieve verification results."
        }
        return verifierStatus
    }

    /**
     * verify the original contract using a conf file.
     *
     */
    private fun verifyOriginal(): JsonElement {
        Logger.always("Verifying original contract", respectQuiet = true)
        val certoraConfigFile = projectFolder().resolve("verify-orig.conf")
        certoraRunParameters.writeTo(certoraConfigFile)
        val origRes = if (certoraRunParameters.hasStaging()) {
            val (_, stagedDest) = verifyOnStaging(certoraConfigFile, projectFolder())
            Json.parseToJsonElement(Files.readString(stagedDest?.resolve("Reports/output.json")))
        } else {
            verifyAndGetResults(certoraConfigFile, projectFolder(), projectFolder())
            Json.parseToJsonElement(Files.readString(getLatestOutputJson(projectFolder())))
        }
        return origRes
    }

    /**
     * Creates a conf file from the bash script to be used
     * for any further argument modifications.
     * */
    private fun getConf(): CertoraRunParameters {
        Logger.always("Generating .conf file from run script...", respectQuiet = true)
        val confPath = projectFolder().resolve("conf-file.conf")
        val exportConfPath = "export CERTORA_DUMP_CONFIG='$confPath'\n\n"
        // TODO: not sure if bash -s works on windows :/
        val (_, _, exitcode) = runCommand(listOf("bash", "-s"), exportConfPath, projectFolder(), runScript().toFile())
        check(exitcode == 0) {"Failed to make conf file from bash script."}
        var certoraRunParameters =  CertoraRunParameters(Json.parseToJsonElement(confPath.toFile().readText()))
        if (staging.useCloud) {
            certoraRunParameters = certoraRunParameters.setStaging(staging.branch)
        }
        Files.delete(confPath)
        return certoraRunParameters
    }

    override fun run() {
        val outputFolderFile = outputFolder().toFile()
        // Initialize all the necessary folders for the mutation report
        if (!outputFolderFile.deleteRecursively()) {
            throw IOException(outputFolderFile.toString())
        }
        Files.createDirectory(outputFolder())
        Files.createDirectory(allFolder())
        Files.createDirectory(compileErrorFolder())
        Files.createDirectory(passedVerificationFolder())
        Files.createDirectory(failedVerificationFolder())

        val allFileASTs = getSourceJsons()

        val origRes = verifyOriginal()
        val results: MutableList<Pair<String, JsonElement>> = mutableListOf()
        results.add("Original" to origRes)

        val rand = Random(seed.hashCode())
        if (manualMutations.size > mutationFiles.size) {
            throw IllegalArgumentException("Expected one manual mutation folder per mutation file, but got ${manualMutations.size} folders")
        }

        val padding = List(mutationFiles.size - manualMutations.size) { null }

        for ((fileString, manualFolder) in mutationFiles.zip(manualMutations + padding)) {
            val manualMutations = if (manualFolder != null) {
                val res = Paths.get(manualFolder)
                if (!(res.exists() && res.isDirectory())) {
                    throw IllegalArgumentException("Manual mutation folder does not exist: ${res.absolutePathString()}")
                }
                res
            } else {
                null
            }

            val fileToMutate = Paths.get(fileString).absolute()
            if (!fileToMutate.exists()) {
                throw IllegalArgumentException("File $fileString does not exist")
            }

            // Generate valid mutants
            val mutationList = mutateFile(fileToMutate, allFileASTs, manualMutations, rand)

            // In parallel, verify mutants
            // The ParallelPool code will attempt to run the verification according the calculated number of
            // processors on the machine (which could be modified to use the number of cores rather than
            // processors), but in the case of the --staging, we want to submit everything to the cloud all at
            // once and not limit the run to the number of processors (all the work is being done in the cloud).
            val actualNumberOfThreads = if (certoraRunParameters.hasStaging()) mutationList.size else numThreads

            results += ParallelPool(actualNumberOfThreads).run(
                mutationList.forkEvery { mutant ->
                    compute {
                        "${mutant.fileName}" to
                                verifyMutant(fileToMutate, mutant)
                    }

                }.pcompute()
            )

            check(results.size == mutationList.size + 1) { "Expected original plus exactly one result per mutation: got ${results.size} vs ${mutationList.size}" }
        }

        // generate a report
        val resultsJson = JsonObject(results.associate {it.first to it.second})
        Files.writeString(resultsJsonFile(), resultsJson.toString())
    }

}

fun main(args: Array<String>) {
    MutationTest().main(args)
}
