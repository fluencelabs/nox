// *****
// *** import `js-fluence-client` to start work with real-time cluster
// *****

import * as fluence from "js-fluence-client"

// *****
// *** create default session with credentials for interaction with the cluster,
// *** you can change the port if you want to interact with another node or your cluster have other ports
// *****

const session = fluence.createDefaultSession("localhost", 25057);

window.onload = function () {

    //select elements and assign them to vars
    const newTask = document.querySelector('#new-task');
    const addTaskBtn = document.querySelector('#addTask');

    const toDoUl = document.querySelector(".todo-list ul");
    const completeUl = document.querySelector(".complete-list ul");

    const todoLoader = document.querySelector("#todo-loader");
    const doneLoader = document.querySelector("#done-loader");

    // *****
    // *** define table names that will store info about tasks
    // *****

    let todoTable = "TODO_LIST";

    // *****
    // *** define a function that will create tables in the llamadb running on the fluence cluster
    // *** `do_query` is a command that accepts queries. It is implemented on top of llamadb in Rust.
    // *** If you wish, you can customize llamadb, compile it to WebAssembly and deploy your own version on Fluence. 
    // *** Toolset for that is still in development though. We will address that on future workshops.
    // *****

    function createTables() {
        const createTodoQuery = `CREATE TABLE ${todoTable}(task varchar(128), done int)`;

        session.invoke("do_query", createTodoQuery).result()
            .then((r) => {console.log("todo table created")});
    }

    createTables();

    // *****
    // *** define a function that will get tasks from the fluence cluster and add it on the board
    // *****

    function loadTasks() {
        const query = `SELECT * FROM ${todoTable}`;
        return session.invoke("do_query", query).result().then((res) => {
            // encode result to string and split by `\n` for list of tasks
            let resultArr = res.asString().split('\n');
            // remove column names element
            resultArr.shift();

            // add all tasks on board
            resultArr.forEach((resultStr) => {
                let result = resultStr.trim();
                if (result && result.size !== 0) {
                    result = result.split(',');
                    if (result[1].trim() === "1") {
                        updateDoneList(result[0]);
                    } else {
                        updateTaskList(result[0])
                    }
                }
            });
        }).then(r => dataLoaded());
    }

    function dataLoaded() {
        addTaskBtn.disabled = false;
        todoLoader.hidden = true;
        doneLoader.hidden = true;
    }

    // *****
    // *** create tables and load all tasks to the board
    // *****

    loadTasks();

    // *****
    // *** insert a task into the fluence cluster
    // *****

    function addTaskToFluence(task, done) {
        if (done) {
            done = "1"
        } else {
            done = "0"
        }
        const query = `INSERT INTO ${todoTable} VALUES ('${task}', ${done})`;
        session.invoke("do_query", query).result().then((r) => {
            console.log("task inserted")
        })
    }

    function updateDone(task, done) {
        if (done) {
            done = "1"
        } else {
            done = "0"
        }
        const query = `UPDATE ${todoTable} SET done = ${done} WHERE task = '${task}'`;
        session.invoke("do_query", query).result().then((res) => {
            console.log("task deleted: " + res.asString());
        })
    }

    // *****
    // *** delete a task from fluence
    // *****

    function deleteFromFluence(task) {
        const query = `DELETE FROM ${todoTable} WHERE task = '${task}'`;
        session.invoke("do_query", query).result().then((res) => {
            console.log("task deleted: " + res.asString());
        })
    }

    //create functions

    //creating the actual task list item
    function createNewTask(task) {
        console.log("Creating task...");

        //set up the new list item
        const listItem = document.createElement("li");
        const checkBox = document.createElement("input");
        const label = document.createElement("label");


        //pull the inputed text into label
        label.innerText = task;

        //add properties
        checkBox.type = "checkbox";

        //add items to the li
        listItem.appendChild(checkBox);
        listItem.appendChild(label);
        //everything put together
        return listItem;
    }

    function createDoneTask(task) {
        console.log("Creating task...");

        //set up the new list item
        const listItem = document.createElement("li");
        const label = document.createElement("label");


        //pull the inputed text into label
        label.innerText = task;

        //add properties
        const deleteBtn = document.createElement("button"); // <button>
        deleteBtn.innerText = "Delete";
        deleteBtn.className = "delete";
        listItem.appendChild(deleteBtn);

        //add items to the li
        listItem.appendChild(label);
        //everything put together
        return listItem;
    }

    function updateTaskList(task) {
        const listItem = createNewTask(task);
        //add the new list item to list
        toDoUl.appendChild(listItem);
        //clear the input
        newTask.value = "";

        //bind the new list item to the incomplete list
        bindIncompleteItems(listItem, completeTask);
    }

    //do the new task into actual incomplete list
    function addTask() {
        const task = newTask.value.trim();
        console.log("Adding task: " + task);

        // *****
        // *** we need to add a task to the fluence cluster after we pressed the `Add task` button
        // *****

        addTaskToFluence(task);

        updateTaskList(task)
    }

    function updateDoneList(task) {
        const listItem = createDoneTask(task);

        //place it inside the completed list
        completeUl.appendChild(listItem);

        //bind the new completed list
        bindCompleteItems(listItem, deleteTask);
    }

    function completeTask() {

        //grab the checkbox's parent element, the <li> it's in
        const listItem = this.parentNode;

        const task = listItem.getElementsByTagName('label')[0].innerText;

        // *****
        // *** when we complete a task, we need to delete it from `todo` table and add to `done` table
        // *****

        updateDone(task, true);

        //create and insert the delete button
        const deleteBtn = document.createElement("button"); // <button>
        deleteBtn.innerText = "Delete";
        deleteBtn.className = "delete";
        listItem.appendChild(deleteBtn);


        //select the checkbox from the completed checkbox and remove it
        const checkBox = listItem.querySelector("input[type=checkbox]");
        checkBox.remove();

        //place it inside the completed list
        completeUl.appendChild(listItem);

        //bind the new completed list
        bindCompleteItems(listItem, deleteTask);
    }

    //delete task functions
    function deleteTask() {
        console.log("Deleting task...");

        const listItem = this.parentNode;
        const ul = listItem.parentNode;

        const task = listItem.getElementsByTagName('label')[0].innerText;

        // *****
        // *** delete a task from the cluster when we press `Delete` button on the completed task
        // *****

        deleteFromFluence(task);

        ul.removeChild(listItem);
    }

    //a function that binds each of the elements of the incomplete list
    function bindIncompleteItems(taskItem, checkBoxClick) {
        //bind the checkbox to a var
        const checkBox = taskItem.querySelector("input[type=checkbox]");

        //setup event listener for the checkbox
        checkBox.onchange = checkBoxClick;
    }


    //a function that binds each of elements in the complete list
    function bindCompleteItems(taskItem, deleteButtonPress) {
        //bind the delete button
        const deleteButton = taskItem.querySelector(".delete");

        //when the delete button is pressed, run the `deleteTask` function
        deleteButton.onclick = deleteButtonPress;

    }

    for (let i = 0; i < toDoUl.children.length; i++) {
        bindIncompleteItems(toDoUl.children[i], completeTask);
    }

    for (let i = 0; i < completeUl.children.length; i++) {
        bindCompleteItems(completeUl.children[i], deleteTask);
    }


    addTaskBtn.addEventListener("click", addTask);
};

const _global = (window /* browser */ || global /* node */);
_global.fluence = fluence;
