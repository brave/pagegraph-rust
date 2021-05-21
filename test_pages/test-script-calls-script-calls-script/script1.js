window.onload = () => {
    let myScript = document.createElement("script");
    myScript.setAttribute("src", "script2.js");
    document.body.appendChild(myScript);

    let anotherScript = document.createElement("script"); 
    anotherScript.setAttribute("src", "https://www.google-analytics.com/analytics.js");
    document.body.appendChild(anotherScript);
}
