function loadTemplate(templateName) {
	let template = document.getElementById(templateName).content.cloneNode(true);
	let elements = [template];

	for (let i = 1; i < arguments.length; i++) {
		let el = template.getElementById(arguments[i]);
		elements.push(el);
	}

	return elements;
}

function setEnabled(element, enabled) {
	if (enabled) {
		element.removeAttribute("disabled");
	} else {
		element.setAttribute("disabled", "disabled");
	}
}

function isValidHomePage(name, room) {
	return name.value.length > 0 && room.value.length > 0;
}

async function loadHomePage() {
	let [html, name, room, join] = loadTemplate("homePage", "name", "room", "join");

	name.oninput = () => setEnabled(join, isValidHomePage(name, room));
	room.oninput = () => setEnabled(join, isValidHomePage(name, room));

	document.body.appendChild(html);
}

const handleLocation = async () => {
	const path = window.location.pathname;

	document.body.innerHTML = "";

	console.log(path);

	switch (path) {
		case "":
		case "/":
			await loadHomePage();
			break;
		default:
			if (path.startsWith("/watch/")) {
				await loadVideoPage();
				break;
			}

			console.log("oops!");
			break;
	}
};

window.onpopstate = handleLocation;

window.onload = () => {
	handleLocation();
};

/* vi: set sw=4 ts=4: */
