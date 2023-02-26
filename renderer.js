const renderCurrentLuminance = async () => {
  const luminanceDiv = document.getElementById("currentLuminances");
  if (luminanceDiv.hasChildNodes()) {
    luminanceDiv.replaceChildren();
  }

  const values = await monitorAPI.getLuminance();
  for (let i = 0; i < values.length; i++) {
    let p = document.createElement("span");
    p.innerText = `monitor ${i + 1}: ${values[i]}`; 
    luminanceDiv.appendChild(p);
    luminanceDiv.appendChild(document.createElement('br'));
  }
};

const setButton = document.getElementById("setButton");
setButton.addEventListener("click", async function () {
  let input = document.getElementById("luminanceValue");
  await monitorAPI.setLuminance(input.value);
  renderCurrentLuminance();
});

renderCurrentLuminance();
