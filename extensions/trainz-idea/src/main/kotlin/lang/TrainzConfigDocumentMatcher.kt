package lang

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.redhat.devtools.lsp4ij.AbstractDocumentMatcher
import org.jetbrains.annotations.NotNull

/**
 * Document matcher that restricts the TrainzConfig language server mapping to
 * files named exactly `config.txt`, avoiding accidental activation on arbitrary
 * plain-text files.
 */
class TrainzConfigDocumentMatcher : AbstractDocumentMatcher() {

    override fun match(@NotNull virtualFile: VirtualFile, @NotNull project: Project): Boolean {
        return virtualFile.name == "config.txt"
    }
}
