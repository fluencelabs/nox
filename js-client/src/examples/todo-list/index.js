import * as fluence from "js-fluence-client"

window.onload = function () {

    let todo_table = "TODO_LIST";
    let done_table = "DONE_LIST";

    // creates deafult session with credentials
    const session = fluence.createDefaultSession("localhost", 29057);

    const getTableName = function(done) {
        if (done) {
            return done_table;
        } else {
            return todo_table;
        }
    };

    // creates the table in llamadb in fluence cluster
    const createTables = function () {
        const createTodoQuery = "CREATE TABLE " + todo_table + "(task varchar(128))";
        const createDoneQuery = "CREATE TABLE " + done_table + "(task varchar(128))";

        const createTodo = session.invoke("do_query", createTodoQuery).result();
        const createDone = session.invoke("do_query", createDoneQuery).result();

        Promise.all([createTodo, createDone])
            .then((r) => {
                console.log("table creation result: " + r.map((v) => v.asString()).join("\n"))
            }).catch((e) => {
            console.log("table creation error: " + e)
        });
    };

    // gets tasks from fluence
    const loadTasks = function(table, func) {
        const query = "SELECT * FROM " + table;
        session.invoke("do_query", query).result().then((res) => {
            let tasks = res.asString().split('\n');
            // removes column names
            tasks.shift();
            console.log("all tasks selection from " + table + ": " + tasks.join(","));
            tasks.forEach((taskStr) => {
                let task = taskStr.trim();
                if (task) {
                    func(task);
                }
            });
        }).catch((e) => {
            console.log("list selection from " + table + " error: " + e)
        });
    };


    // insert a task into fluence
    const addTaskToFluence = function (task, done) {
        const insertion = "('" + task + "')";
        let table = getTableName(done);

        const query = "insert into " + table + " values " + insertion;
        session.invoke("do_query", query).result().then((r) => {
            console.log("insertion result: " + r.asString())
        })
    };

    const deleteFromFluence = function (task, done) {
        let table = getTableName(done);
        const query = "DELETE FROM " + table + " WHERE TASK = '" + task +"'";
        session.invoke("do_query", query).result().then((res) => {
            console.log("task deleted: " + res.asString());
        }).catch((e) => {
            console.log("error on task deletion: " + JSON.stringify(e))
        });
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

    const createDoneTask = function (task) {
        console.log("Creating task...");

        //SET UP THE NEW LIST ITEM
        const listItem = document.createElement("li");
        const label = document.createElement("label");


        //PULL THE INPUTED TEXT INTO LABEL
        label.innerText = task;

        //ADD PROPERTIES
        const deleteBtn = document.createElement("button"); // <button>
        deleteBtn.innerText = "Delete";
        deleteBtn.className = "delete";
        listItem.appendChild(deleteBtn);

        //ADD ITEMS TO THE LI
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
        const task = newTask.value.trim();
        console.log("Adding task: " + task);
        addTaskToFluence(task, false);
        updateTaskList(task)
    };

    const updateDoneList = function (task) {
        const listItem = createDoneTask(task);

        //PLACE IT INSIDE THE COMPLETED LIST
        completeUl.appendChild(listItem);

        //BIND THE NEW COMPLETED LIST
        bindCompleteItems(listItem, deleteTask);
    };

    const completeTask = function () {

        //GRAB THE CHECKBOX'S PARENT ELEMENT, THE LI IT'S IN
        const listItem = this.parentNode;

        const task = listItem.getElementsByTagName('label')[0].innerText;

        deleteFromFluence(task, false);
        addTaskToFluence(task, true);

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

        const task = listItem.getElementsByTagName('label')[0].innerText;
        deleteFromFluence(task, true);

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

    createTables();
    loadTasks(todo_table, updateTaskList);
    loadTasks(done_table, updateDoneList);

    for (let i = 0; i < toDoUl.children.length; i++) {
        bindIncompleteItems(toDoUl.children[i], completeTask);
    }

    for (let i = 0; i < completeUl.children.length; i++) {
        bindCompleteItems(completeUl.children[i], deleteTask);
    }


    addTaskBtn.addEventListener("click", addTask);
};