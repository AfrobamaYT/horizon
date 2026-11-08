import java.io.*;
import java.nio.file.*;
import org.bukkit.Bukkit;
import org.bukkit.event.*;
import org.bukkit.event.player.*;
import org.bukkit.event.world.ChunkLoadEvent;
import org.bukkit.plugin.java.JavaPlugin;

/** Public API observer; join/quit markers bound the comparison window. */
public final class PaperTrace extends JavaPlugin implements Listener {
    private BufferedWriter output;
    @Override public void onEnable() {
        try {
            output = Files.newBufferedWriter(Path.of(System.getProperty("paper.trace")));
        } catch (IOException e) { throw new UncheckedIOException(e); }
        Bukkit.getPluginManager().registerEvents(this, this);
    }
    private void emit(String name, String fields) {
        try {
            output.write("{\"tick\":" + Bukkit.getCurrentTick() + ",\"name\":\"" + name
                + "\",\"fields\":" + fields + ",\"cancelled\":false}\n");
            output.flush();
        } catch (IOException e) { throw new UncheckedIOException(e); }
    }
    @EventHandler(priority = EventPriority.MONITOR)
    public void chunk(ChunkLoadEvent e) {
        emit("ChunkLoadEvent", "{\"world\":\"" + e.getWorld().getKey()
            + "\",\"x\":" + e.getChunk().getX() + ",\"z\":" + e.getChunk().getZ()
            + ",\"new\":" + e.isNewChunk() + "}");
    }
    @EventHandler public void join(PlayerJoinEvent e) { emit("_join", "{}"); }
    @EventHandler public void quit(PlayerQuitEvent e) { emit("_quit", "{}"); }
    @Override public void onDisable() {
        try { if (output != null) output.close(); }
        catch (IOException e) { throw new UncheckedIOException(e); }
    }
}
