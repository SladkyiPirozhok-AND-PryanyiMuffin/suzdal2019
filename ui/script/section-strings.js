const setupStrings = () => {
	const strings = cardData.data_pack.strings
	return div([
		div({class: 'section-toolbar'}, [
			button({id: 'import-strings-button', class: 'text', onclick: importStrings}, 'Import Strings'),
		]),
		table([
			thead([tr([
				th('ID'),
				th('Unknown 1'),
				th('Unknown 2'),
				th('Unknown 3'),
				th('Value')
			])]),
			tbody(strings.map(str => tr({id: `string-${str.id.entity_id}`}, [
				th(str.id.entity_id),
				td('#' + formatHexCode(str.unknown1)),
				td(str.unknown2),
				td(str.unknown3),
				td(str.value)
			])))
		])
	])
}

const viewStrings = () => {
	selectSection('view-strings-button')
	contents.append(sections.strings)
}

const importStrings = () => {
	tauri_invoke('import_strings')
}

const setupMenuStrings = () => {
	const strings = cardData.menu_strings
	return div([
		div({class: 'section-toolbar'}, [
			button({id: 'import-menu-strings-button', class: 'text', onclick: importMenuStrings}, 'Import Strings'),
		]),
		table([
			thead([tr([
				th('ID'),
				th('Value')
			])]),
			tbody(strings.map((str, i) => tr({id: `menu-string-${i}`}, [
				th(i),
				td(str)
			])))
		])
	])
}

const viewMenuStrings = () => {
	selectSection('view-menu-strings-button')
	contents.append(sections.menu_strings)
}

const importMenuStrings = () => {
	tauri_invoke('import_menu_strings')
}
