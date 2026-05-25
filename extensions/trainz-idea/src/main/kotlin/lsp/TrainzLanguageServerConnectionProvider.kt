package lsp

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.ide.util.PropertiesComponent
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.installation.CommandLineUpdater
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider
import org.jetbrains.annotations.NotNull
import settings.TrainzLanguageServerSettings

/**
 * Starts the Trainz language server binary as a child process.
 *
 * Exposes a [commandLineUpdater] so the LSP4IJ declarative installer can call
 * [CommandLineUpdater.setCommandLine] on the live instance when installation
 * completes, updating both the in-memory [GeneralCommandLine] and persisting
 * the path to [PropertiesComponent] for future IDE sessions.
 *
 * The binary path is resolved in priority order:
 * 1. Path persisted by the LSP4IJ installer (stored in [PropertiesComponent])
 * 2. `trainz.lsp.server.path` JVM system property (useful for development)
 * 3. `trainz-language-server` on the system PATH (fallback)
 *
 * CLI flags are populated from [TrainzLanguageServerSettings] on each launch.
 */
class TrainzLanguageServerConnectionProvider(@NotNull project: Project) : OSProcessStreamConnectionProvider() {

    companion object {
        /** [PropertiesComponent] key used to persist the installed server binary path. */
        const val COMMAND_LINE_KEY = "trainz.lsp.server.commandLine"
        private const val DEFAULT_COMMAND = "trainz-language-server"

        /** Reads the persisted server path, falling back to the system property or default. */
        fun resolveServerPath(): String {
            val raw = PropertiesComponent.getInstance().getValue(COMMAND_LINE_KEY)
                ?: System.getProperty("trainz.lsp.server.path", DEFAULT_COMMAND)
            return raw.replace("\$USER_HOME\$", System.getProperty("user.home"))
        }

        /**
         * Builds a [GeneralCommandLine] for the language server, applying the exe path
         * and all CLI flags derived from [TrainzLanguageServerSettings].
         */
        fun buildCommandLine(exePath: String): GeneralCommandLine {
            val cli = GeneralCommandLine()
            cli.withExePath(exePath)

            val settings = TrainzLanguageServerSettings.getInstance()

            if (settings.validationPath.isNotBlank()) {
                cli.addParameters("--validation-path", settings.validationPath.trim())
            }
            if (settings.searchPaths.isNotBlank()) {
                cli.addParameters("--search-paths", settings.searchPaths.trim())
            }
            if (settings.logFile.isNotBlank()) {
                cli.addParameters("--log-file", settings.logFile.trim())
            }
            if (settings.logLevel.isNotBlank()) {
                cli.addParameters("--log-level", settings.logLevel.trim())
            }
            if (settings.assetCache.isNotBlank()) {
                cli.addParameters("--asset-cache", settings.assetCache.trim())
            }
            if (settings.tdxCache.isNotBlank()) {
                cli.addParameters("--tdx-cache", settings.tdxCache.trim())
            }
            if (settings.extensionsOverrides.isNotBlank()) {
                cli.addParameters("--extensions-overrides", settings.extensionsOverrides.trim())
            }

            return cli
        }
    }

    init {
        super.setCommandLine(buildCommandLine(resolveServerPath()))
    }

    /**
     * [CommandLineUpdater] backed by this connection provider.
     *
     * Pass this to [TrainzLanguageServerInstaller] so the installer's
     * `onSuccess.configureServer` step calls [CommandLineUpdater.setCommandLine]
     * here, which updates the live [GeneralCommandLine] **and** persists the path
     * for future startups.
     */
    val commandLineUpdater: CommandLineUpdater = object : CommandLineUpdater {
        override fun getCommandLine(): String =
            this@TrainzLanguageServerConnectionProvider.commandLine?.exePath ?: resolveServerPath()

        override fun setCommandLine(newPath: String) {
            val resolvedPath = newPath.replace("\$USER_HOME\$", System.getProperty("user.home"))
            PropertiesComponent.getInstance().setValue(COMMAND_LINE_KEY, resolvedPath)
            this@TrainzLanguageServerConnectionProvider.commandLine = buildCommandLine(resolvedPath)
        }
    }
}