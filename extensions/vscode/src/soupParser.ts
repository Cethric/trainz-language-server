export interface SoupSection {
    key: string;
    children: (SoupSection | SoupKeyValue)[];
}

export interface SoupKeyValue {
    key: string;
    value: string;
}

export type SoupNode = SoupSection | SoupKeyValue;

export function isSection(node: SoupNode): node is SoupSection {
    return 'children' in node;
}

export function isKeyValue(node: SoupNode): node is SoupKeyValue {
    return 'value' in node;
}

/**
 * Parses SOUP text format into a hierarchical structure
 * SOUP format example:
 * section1 {
 *     key1 value1
 *     subsection {
 *         key2 value2
 *     }
 * }
 */
export function parseSoup(text: string): SoupNode[] {
    const lines = text.split('\n');
    const result: SoupNode[] = [];
    const stack: { nodes: SoupNode[]; indent: number }[] = [{ nodes: result, indent: -1 }];

    for (const line of lines) {
        const trimmed = line.trim();
        if (trimmed === '') {
            continue;
        }

        // Calculate indent level
        const indentMatch = line.match(/^(\s*)/);
        const indent = indentMatch ? indentMatch[1].length : 0;

        // Pop stack until we find the correct parent level
        while (stack.length > 1 && stack[stack.length - 1].indent >= indent) {
            stack.pop();
        }

        const currentParent = stack[stack.length - 1].nodes;

        if (trimmed.endsWith('{')) {
            // Section start
            const match = line.match(/^(\s*)([^"\s]+|"[^"]+")\s*{$/);
            if (match) {
                const [, , key] = match;
                const section: SoupSection = {
                    key: key.replace(/^"(.*)"$/, '$1'), // Remove quotes if present
                    children: []
                };
                currentParent.push(section);
                stack.push({ nodes: section.children, indent });
            }
        } else if (trimmed === '}') {
            // Section end - handled by indent logic above
            continue;
        } else {
            // Key-value pair
            const match = line.match(/^(\s*)([^"\s]+|"[^"]+")\s+(.+)$/);
            if (match) {
                const [, , key, value] = match;
                const kv: SoupKeyValue = {
                    key: key.replace(/^"(.*)"$/, '$1'), // Remove quotes if present
                    value: value
                };
                currentParent.push(kv);
            }
        }
    }

    return result;
}

/**
 * Converts parsed SOUP structure back to text format
 */
export function serializeSoup(nodes: SoupNode[], indent: string = ''): string {
    const lines: string[] = [];

    for (const node of nodes) {
        if (isSection(node)) {
            lines.push(`${indent}${node.key} {`);
            lines.push(serializeSoup(node.children, indent + '    '));
            lines.push(`${indent}}`);
        } else if (isKeyValue(node)) {
            lines.push(`${indent}${node.key} ${node.value}`);
        }
    }

    return lines.join('\n');
}

/**
 * Updates a key-value pair in the parsed SOUP structure
 */
export function updateKeyValue(nodes: SoupNode[], keyPath: string[], newValue: string): boolean {
    function findAndUpdate(currentNodes: SoupNode[], pathIndex: number): boolean {
        for (const node of currentNodes) {
            if (isSection(node)) {
                if (pathIndex < keyPath.length && node.key === keyPath[pathIndex]) {
                    if (pathIndex === keyPath.length - 1) {
                        // This shouldn't happen - sections don't have values
                        return false;
                    }
                    return findAndUpdate(node.children, pathIndex + 1);
                }
            } else if (isKeyValue(node)) {
                if (pathIndex === keyPath.length - 1 && node.key === keyPath[pathIndex]) {
                    node.value = newValue;
                    return true;
                }
            }
        }
        return false;
    }

    return findAndUpdate(nodes, 0);
}