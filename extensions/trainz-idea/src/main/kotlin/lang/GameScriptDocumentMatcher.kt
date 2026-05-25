package lang

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.redhat.devtools.lsp4ij.AbstractDocumentMatcher
import org.jetbrains.annotations.NotNull

/**
 * Document matcher that restricts the GameScript language server mapping to
 * files with the `.gs` extension.
 */
class GameScriptDocumentMatcher : AbstractDocumentMatcher() {

    override fun match(@NotNull virtualFile: VirtualFile, @NotNull project: Project): Boolean {
        return virtualFile.extension == "gs"
    }
}
