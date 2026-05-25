package settings

import com.intellij.openapi.fileChooser.FileChooser
import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.options.Configurable
import com.intellij.openapi.ui.TextBrowseFolderListener
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import com.intellij.ui.ToolbarDecorator
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBList
import com.intellij.util.ui.FormBuilder
import javax.swing.DefaultListModel
import javax.swing.JComboBox
import javax.swing.JComponent
import javax.swing.JPanel

/**
 * Settings configurable for the Trainz Language Server.
 *
 * Shown under **Settings → Languages & Frameworks → Trainz Language Server**.
 * Each field corresponds to a CLI flag passed to the `trainz-language-server` binary.
 */
class TrainzLanguageServerSettingsConfigurable : Configurable {

    private var panel: JPanel? = null

    private val validationPathField = TextFieldWithBrowseButton()
    private val searchPathsModel = DefaultListModel<String>()
    private val searchPathsList = JBList(searchPathsModel)
    private val logFileField = TextFieldWithBrowseButton()
    private val logLevelCombo = JComboBox(arrayOf("", "error", "warn", "info", "debug", "trace"))
    private val assetCacheField = TextFieldWithBrowseButton()
    private val tdxCacheField = TextFieldWithBrowseButton()
    private val extensionsOverridesField = TextFieldWithBrowseButton()

    override fun getDisplayName(): String = "Trainz Language Server"

    override fun createComponent(): JComponent {
        validationPathField.addBrowseFolderListener(
            TextBrowseFolderListener(FileChooserDescriptorFactory.createSingleFolderDescriptor())
        )
        logFileField.addBrowseFolderListener(
            TextBrowseFolderListener(FileChooserDescriptorFactory.createSingleFileOrExecutableAppDescriptor())
        )
        assetCacheField.addBrowseFolderListener(
            TextBrowseFolderListener(FileChooserDescriptorFactory.createSingleFileDescriptor())
        )
        tdxCacheField.addBrowseFolderListener(
            TextBrowseFolderListener(FileChooserDescriptorFactory.createSingleFolderDescriptor())
        )
        extensionsOverridesField.addBrowseFolderListener(
            TextBrowseFolderListener(FileChooserDescriptorFactory.createSingleFolderDescriptor())
        )

        val searchPathsDecorated = ToolbarDecorator.createDecorator(searchPathsList)
            .setAddAction {
                val descriptor = FileChooserDescriptorFactory.createSingleFolderDescriptor()
                descriptor.title = "Select Search Path"
                val chosen = FileChooser.chooseFile(descriptor, null, null)
                if (chosen != null) {
                    searchPathsModel.addElement(chosen.path)
                }
            }
            .setRemoveAction {
                searchPathsList.selectedIndices.reversed().forEach { searchPathsModel.remove(it) }
            }
            .disableUpDownActions()
            .createPanel()

        panel = FormBuilder.createFormBuilder()
            .addLabeledComponent(
                JBLabel("Validation path (-p):"),
                validationPathField, true
            )
            .addTooltip("Path to the directory containing ACS Text validators")
            .addLabeledComponent(
                JBLabel("Search paths (-s):"),
                searchPathsDecorated, true
            )
            .addTooltip("Directories to search for Trainz scripts (passed as semicolon-separated list to --search-paths)")
            .addLabeledComponent(
                JBLabel("Log file (--log-file):"),
                logFileField, true
            )
            .addTooltip("Path to the log file; leave empty to log to stderr")
            .addLabeledComponent(
                JBLabel("Log level (--log-level):"),
                logLevelCombo, true
            )
            .addTooltip("Verbosity level: error, warn, info, debug, trace")
            .addLabeledComponent(
                JBLabel("Asset cache (--asset-cache):"),
                assetCacheField, true
            )
            .addTooltip("Path to the asset cache SQLite file")
            .addLabeledComponent(
                JBLabel("TDX cache (--tdx-cache):"),
                tdxCacheField, true
            )
            .addTooltip("Path to the TDX asset cache directory")
            .addLabeledComponent(
                JBLabel("Extensions overrides (--extensions-overrides):"),
                extensionsOverridesField, true
            )
            .addTooltip("Path to a folder defining extension overrides")
            .addComponentFillVertically(JPanel(), 0)
            .panel

        return panel!!
    }

    /** Returns the current list contents as a semicolon-joined string. */
    private fun searchPathsAsString(): String =
        (0 until searchPathsModel.size()).map { searchPathsModel.getElementAt(it) }.joinToString(";")

    override fun isModified(): Boolean {
        val s = TrainzLanguageServerSettings.getInstance()
        return validationPathField.text != s.validationPath
                || searchPathsAsString() != s.searchPaths
                || logFileField.text != s.logFile
                || (logLevelCombo.selectedItem as? String ?: "") != s.logLevel
                || assetCacheField.text != s.assetCache
                || tdxCacheField.text != s.tdxCache
                || extensionsOverridesField.text != s.extensionsOverrides
    }

    override fun apply() {
        val s = TrainzLanguageServerSettings.getInstance()
        s.validationPath = validationPathField.text.trim()
        s.searchPaths = searchPathsAsString()
        s.logFile = logFileField.text.trim()
        s.logLevel = (logLevelCombo.selectedItem as? String ?: "").trim()
        s.assetCache = assetCacheField.text.trim()
        s.tdxCache = tdxCacheField.text.trim()
        s.extensionsOverrides = extensionsOverridesField.text.trim()
    }

    override fun reset() {
        val s = TrainzLanguageServerSettings.getInstance()
        validationPathField.text = s.validationPath
        searchPathsModel.clear()
        s.searchPaths.split(";").map { it.trim() }.filter { it.isNotBlank() }
            .forEach { searchPathsModel.addElement(it) }
        logFileField.text = s.logFile
        logLevelCombo.selectedItem = s.logLevel
        assetCacheField.text = s.assetCache
        tdxCacheField.text = s.tdxCache
        extensionsOverridesField.text = s.extensionsOverrides
    }

    override fun disposeUIResources() {
        panel = null
    }
}
