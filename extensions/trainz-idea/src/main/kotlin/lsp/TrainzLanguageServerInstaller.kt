package lsp

import com.intellij.openapi.progress.ProgressIndicator
import com.redhat.devtools.lsp4ij.installation.CommandLineUpdater
import com.redhat.devtools.lsp4ij.installation.DeclarativeLanguageServerInstaller
import com.redhat.devtools.lsp4ij.installation.definition.InstallerContext
import com.redhat.devtools.lsp4ij.installation.definition.ServerInstallerDescriptor
import com.redhat.devtools.lsp4ij.installation.definition.ServerInstallerManager

/**
 * Declarative installer for the Trainz Language Server.
 *
 * Loads the installer descriptor from the bundled `lsp/installer.json` resource,
 * which defines how to check for and download the language server binary from GitHub.
 *
 * Accepts an optional [CommandLineUpdater] (typically the [TrainzLanguageServerConnectionProvider] instance's
 * [TrainzLanguageServerConnectionProvider.commandLineUpdater]) so that when the installer's
 * `onSuccess.configureServer` step resolves the binary path it is forwarded to the
 * live connection provider and persisted for future IDE startups.
 */
class TrainzLanguageServerInstaller(
    private val commandLineUpdater: CommandLineUpdater? = null
) : DeclarativeLanguageServerInstaller() {

    private val descriptor: ServerInstallerDescriptor? by lazy {
        val json = javaClass.getResourceAsStream("/lsp/installer.json")
            ?.bufferedReader()
            ?.readText()
            ?: return@lazy null
        ServerInstallerManager.getInstance().loadInstaller(json)
    }

    override fun getServerInstallerDescriptor(): ServerInstallerDescriptor? = descriptor

    override fun createInstallerContext(
        action: InstallerContext.InstallerAction,
        indicator: ProgressIndicator
    ): InstallerContext {
        val context = super.createInstallerContext(action, indicator)
        context.commandLineUpdater = commandLineUpdater
        return context
    }
}
