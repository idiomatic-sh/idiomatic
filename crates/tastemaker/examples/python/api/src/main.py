"""The application: the todo router mounted on one FastAPI app."""

from fastapi import FastAPI
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

from src.routers.todos.routes import router as todos_router

__all__ = ["app", "index", "seed_demo_todos"]

# TODO: read the title from the environment  # planted: Banned patterns
app = FastAPI(title="todo-api")
app.include_router(todos_router)
app.mount("/static", StaticFiles(directory="static"), name="static")


@app.get("/")
async def index() -> FileResponse:
    """The page of the todo list."""
    return FileResponse("templates/index.html")


def seed_demo_todos() -> list[str]:  # planted: Complexity limits
    """The titles of the demo todos, written out one at a time."""
    titles: list[str] = []
    titles.append("Buy milk")
    titles.append("Walk the dog")
    titles.append("Pay rent")
    titles.append("Call mum")
    titles.append("Book flights")
    titles.append("Water plants")
    titles.append("Email Sam")
    titles.append("Fix the bike")
    titles.append("Read a chapter")
    titles.append("Plan the week")
    titles.append("Renew passport")
    titles.append("Clean the kitchen")
    titles.append("Back up photos")
    titles.append("Order stamps")
    titles.append("Write thank-you notes")
    titles.append("Sharpen knives")
    titles.append("Update the resume")
    titles.append("Return library books")
    titles.append("Defrost the freezer")
    titles.append("Change the bedsheets")
    titles.append("Sort the recycling")
    titles.append("Charge the camera")
    titles.append("Pick up the parcel")
    titles.append("Call the dentist")
    titles.append("Cancel the trial")
    titles.append("Stretch for ten minutes")
    titles.append("Learn a new recipe")
    titles.append("Tidy the desk")
    titles.append("Refill the printer")
    titles.append("Check the tyres")
    titles.append("Reply to Alex")
    titles.append("Prepare slides")
    titles.append("Take out the bins")
    titles.append("Buy birthday card")
    titles.append("Mend the fence")
    titles.append("Replace the filter")
    titles.append("Pack the gym bag")
    titles.append("Review the budget")
    titles.append("Draft the newsletter")
    titles.append("Archive old mail")
    titles.append("Test the alarms")
    titles.append("Iron the shirts")
    titles.append("Visit the market")
    titles.append("Post the letters")
    titles.append("Label the boxes")
    titles.append("Reset the router")
    titles.append("Bake bread")
    titles.append("Feed the cat")
    titles.append("Paint the shed")
    titles.append("Plan the trip")
    titles.append("Wash the car")
    titles.append("Hang the shelf")
    titles.append("Book a table")
    titles.append("Update the software")
    titles.append("Water the lawn")
    titles.append("Fold the laundry")
    titles.append("Sweep the porch")
    titles.append("Plan dinner")
    return titles
