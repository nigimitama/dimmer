const luminances = document.getElementById("luminances");
luminances.innerText = monitorAPI.getLuminance();

const luminanceValue = document.getElementById("luminanceValue");
luminanceValue.addEventListener("change", function (event) {
    monitorAPI.setLuminance(event.target.value);
})
