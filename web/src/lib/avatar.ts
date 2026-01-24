/** Up to two initials, e.g. "Maya Lopez" -> "ML", "jonas" -> "J". */
export function initials(name: string): string {
	const parts = name.trim().split(/\s+/).filter(Boolean);
	if (parts.length === 0) return '?';
	const letters = parts.length === 1 ? [parts[0]] : [parts[0], parts[parts.length - 1]];
	return letters.map((p) => p[0].toUpperCase()).join('');
}

/** A stable hue for a name, so a person keeps their colour everywhere. */
export function hue(name: string): number {
	let hash = 0;
	for (const char of name) hash = (hash * 31 + char.codePointAt(0)!) % 360;
	return hash;
}
