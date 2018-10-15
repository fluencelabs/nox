import * as fluence from "js-fluence-client"

window.onload = function () {

    let size = 0;

    // creates deafult session with credentials
    const session = fluence.createDefaultSession("localhost", 25057);

    // creates the table in llamadb in fluence cluster
    session.invoke("do_query", ["\"CREATE TABLE TODO_LIST(id int, task varchar(128), deleted int)\""])
        .result().then((r) => {
        console.log("table creation result: " + r.asString())
    }).catch((e) => {
        console.log("table creation error: " + JSON.stringify(e))
    });

    // gets tasks from fluence
    session.invoke("do_query", ["\"SELECT * FROM TODO_LIST\""]).result().then((res) => {
        console.log("all tasks selection: " + JSON.stringify(res.asString()));
        let tasks = res.asString().split('\n');
        // removes column names
        tasks.shift();
        tasks.forEach((taskStr) => {
            updateTaskList(taskStr.trim().split(",")[1]);
        });
        size = tasks.length
    }).catch((e) => {
        console.log("todo list selection error: " + JSON.stringify(e))
    });

    // insert a task into fluence
    const addTaskToFluence = function(task) {
        const taskId = size++;
        const insertion = "(" + taskId + ", '" + task + "', 0)";
        const query = "\"insert into TODO_LIST values " + insertion + "\"";
        session.invoke("do_query", [query]).result().then((r) => {
            console.log("insertion result: " + r.asString())
        })
    };


    //SELECT ELEMENTS AND ASSIGN THEM TO VARS
    const newTask = document.querySelector('#new-task');
    const addTaskBtn = document.querySelector('#addTask');

    const toDoUl = document.querySelector(".todo-list ul");
    const completeUl = document.querySelector(".complete-list ul");


    //CREATE FUNCTIONS

    //CREATING THE ACTUAL TASK LIST ITEM
    const createNewTask = function (task) {
        console.log("Creating task...");

        //SET UP THE NEW LIST ITEM
        const listItem = document.createElement("li");
        const checkBox = document.createElement("input");
        const label = document.createElement("label");


        //PULL THE INPUTED TEXT INTO LABEL
        label.innerText = task;

        //ADD PROPERTIES
        checkBox.type = "checkbox";

        //ADD ITEMS TO THE LI
        listItem.appendChild(checkBox);
        listItem.appendChild(label);
        //EVERYTHING PUT TOGETHER
        return listItem;

    };

    const updateTaskList = function (task) {
        const listItem = createNewTask(task);
        //ADD THE NEW LIST ITEM TO LIST
        toDoUl.appendChild(listItem);
        //CLEAR THE INPUT
        newTask.value = "";

        //BIND THE NEW LIST ITEM TO THE INCOMPLETE LIST
        bindIncompleteItems(listItem, completeTask);
    };

    //ADD THE NEW TASK INTO ACTUAL INCOMPLETE LIST
    const addTask = function () {
        console.log("Adding task...");
        addTaskToFluence(newTask.value);
        updateTaskList(newTask.value)
    };

    const completeTask = function () {

        //GRAB THE CHECKBOX'S PARENT ELEMENT, THE LI IT'S IN
        const listItem = this.parentNode;

        //CREATE AND INSERT THE DELETE BUTTON
        const deleteBtn = document.createElement("button"); // <button>
        deleteBtn.innerText = "Delete";
        deleteBtn.className = "delete";
        listItem.appendChild(deleteBtn);

        //SELECT THE CHECKBOX FROM THE COMPLETED CHECKBOX AND REMOVE IT
        const checkBox = listItem.querySelector("input[type=checkbox]");
        checkBox.remove();

        //PLACE IT INSIDE THE COMPLETED LIST
        completeUl.appendChild(listItem);

        //BIND THE NEW COMPLETED LIST
        bindCompleteItems(listItem, deleteTask);

    };

    //DELETE TASK FUNCTIONS
    const deleteTask = function () {
        console.log("Deleting task...");

        const listItem = this.parentNode;
        const ul = listItem.parentNode;

        ul.removeChild(listItem);

    };

    //A FUNCTION THAT BINDS EACH OF THE ELEMENTS THE INCOMPLETE LIST

    const bindIncompleteItems = function (taskItem, checkBoxClick) {
        console.log("Binding the incomplete list...");

        //BIND THE CHECKBOX TO A VAR
        const checkBox = taskItem.querySelector("input[type=checkbox]");

        //SETUP EVENT LISTENER FOR THE CHECKBOX
        checkBox.onchange = checkBoxClick;
    };


    //A FUNCTION THAT BINDS EACH OF THE ELEMTS IN THE COMPLETE LIST
    const bindCompleteItems = function (taskItem, deleteButtonPress) {
        console.log("Binding the complete list...");

        //BIND THE DELETE BUTTON
        const deleteButton = taskItem.querySelector(".delete");

        //WHEN THE DELETE BUTTIN IS PRESSED, RUN THE deleteTask function
        deleteButton.onclick = deleteButtonPress;

    };


    for (let i = 0; i < toDoUl.children.length; i++) {
        bindIncompleteItems(toDoUl.children[i], completeTask);
    }

    for (let i = 0; i < completeUl.children.length; i++) {
        bindCompleteItems(completeUl.children[i], deleteTask);
    }


    addTaskBtn.addEventListener("click", addTask);
};